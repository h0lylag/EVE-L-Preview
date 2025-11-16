//! GUI manager implemented with egui/eframe and tray-item system tray support

use std::io::Cursor;
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use eframe::{egui, CreationContext, NativeOptions};
use tracing::{error, info, warn};
use tray_item::{IconSource, TrayItem};

use super::constants::*;

#[derive(Debug, Clone, Copy)]
enum TrayCommand {
    ToggleWindow,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DaemonStatus {
    Starting,
    Running,
    Stopped,
    Crashed(Option<i32>),
}

impl DaemonStatus {
    fn color(&self) -> egui::Color32 {
        match self {
            DaemonStatus::Running => STATUS_RUNNING,
            DaemonStatus::Starting => STATUS_STARTING,
            _ => STATUS_STOPPED,
        }
    }

    fn label(&self) -> String {
        match self {
            DaemonStatus::Running => "\u{25CF}  Running".to_string(),
            DaemonStatus::Starting => "\u{25CF}  Starting...".to_string(),
            DaemonStatus::Stopped => "\u{25CF}  Stopped".to_string(),
            DaemonStatus::Crashed(code) => match code {
                Some(code) => format!("\u{25CF}  Crashed (exit {code})"),
                None => "\u{25CF}  Crashed".to_string(),
            },
        }
    }
}

struct StatusMessage {
    text: String,
    color: egui::Color32,
}

struct ManagerApp {
    _tray: Option<TrayItem>,
    tray_rx: Receiver<TrayCommand>,
    daemon: Option<Child>,
    daemon_status: DaemonStatus,
    last_health_check: Instant,
    status_message: Option<StatusMessage>,
    window_visible: bool,
    allow_close: bool,
}

impl ManagerApp {
    fn new(_cc: &CreationContext<'_>) -> Self {
        info!("Initializing egui manager");

        let (tray, tray_rx) = match create_tray_icon() {
            Ok(result) => result,
            Err(err) => {
                error!(error = ?err, "Failed to initialize tray icon");
                let (_tx, rx) = mpsc::sync_channel(1);
                (None, rx)
            }
        };

        let mut app = Self {
            _tray: tray,
            tray_rx,
            daemon: None,
            daemon_status: DaemonStatus::Stopped,
            last_health_check: Instant::now(),
            status_message: None,
            window_visible: true,
            allow_close: false,
        };

        if let Err(err) = app.start_daemon() {
            error!(error = ?err, "Failed to start preview daemon");
            app.status_message = Some(StatusMessage {
                text: format!("Failed to start daemon: {err}"),
                color: STATUS_STOPPED,
            });
        }

        app
    }

    fn start_daemon(&mut self) -> Result<()> {
        if self.daemon.is_some() {
            return Ok(());
        }

        let child = spawn_preview_daemon()?;
        let pid = child.id();
        info!(pid, "Started preview daemon");

        self.daemon = Some(child);
        self.daemon_status = DaemonStatus::Starting;
        self.status_message = Some(StatusMessage {
            text: format!("Preview daemon starting (PID: {pid})"),
            color: STATUS_STARTING,
        });
        Ok(())
    }

    fn stop_daemon(&mut self) -> Result<()> {
        if let Some(mut child) = self.daemon.take() {
            info!(pid = child.id(), "Stopping preview daemon");
            let _ = child.kill();
            let status = child
                .wait()
                .context("Failed to wait for preview daemon exit")?;
            self.daemon_status = if status.success() {
                DaemonStatus::Stopped
            } else {
                DaemonStatus::Crashed(status.code())
            };
            self.status_message = Some(StatusMessage {
                text: "Preview daemon stopped".to_string(),
                color: STATUS_STOPPED,
            });
        }
        Ok(())
    }

    fn restart_daemon(&mut self) {
        info!("Restart requested from UI");
        if let Err(err) = self.stop_daemon().and_then(|_| self.start_daemon()) {
            error!(error = ?err, "Failed to restart daemon");
            self.status_message = Some(StatusMessage {
                text: format!("Restart failed: {err}"),
                color: STATUS_STOPPED,
            });
        }
    }

    fn poll_daemon(&mut self) {
        if self.last_health_check.elapsed() < Duration::from_millis(DAEMON_CHECK_INTERVAL_MS) {
            return;
        }
        self.last_health_check = Instant::now();

        if let Some(child) = self.daemon.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    warn!(pid = child.id(), exit = ?status.code(), "Preview daemon exited");
                    self.daemon = None;
                    self.daemon_status = if status.success() {
                        DaemonStatus::Stopped
                    } else {
                        DaemonStatus::Crashed(status.code())
                    };
                    self.status_message = Some(StatusMessage {
                        text: "Preview daemon exited".to_string(),
                        color: STATUS_STOPPED,
                    });
                }
                Ok(None) => {
                    if matches!(self.daemon_status, DaemonStatus::Starting) {
                        self.daemon_status = DaemonStatus::Running;
                        self.status_message = Some(StatusMessage {
                            text: "Preview daemon running".to_string(),
                            color: STATUS_RUNNING,
                        });
                    }
                }
                Err(err) => {
                    error!(error = ?err, "Failed to query daemon status");
                }
            }
        }
    }

    fn process_tray_commands(&mut self, ctx: &egui::Context) {
        while let Ok(command) = self.tray_rx.try_recv() {
            match command {
                TrayCommand::ToggleWindow => {
                    if self.window_visible {
                        self.hide_window(ctx);
                    } else {
                        self.show_window(ctx);
                    }
                }
                TrayCommand::Quit => {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn hide_window(&mut self, ctx: &egui::Context) {
        if !self.window_visible {
            return;
        }
        info!("Hiding manager window to tray");
        self.window_visible = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        if self.window_visible {
            return;
        }
        info!("Showing manager window from tray");
        self.window_visible = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
}

impl eframe::App for ManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.allow_close {
                info!("Close requested - shutting down");
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.hide_window(ctx);
                return;
            }
        }

        self.process_tray_commands(ctx);
        self.poll_daemon();

        if !self.window_visible {
            ctx.request_repaint_after(Duration::from_millis(DAEMON_CHECK_INTERVAL_MS));
            return;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(PADDING);
            ui.heading("EVE-L Preview Manager");
            ui.add_space(SECTION_SPACING);

            ui.group(|ui| {
                ui.label(egui::RichText::new("Daemon Status").strong());
                ui.colored_label(self.daemon_status.color(), self.daemon_status.label());
                if let Some(child) = &self.daemon {
                    ui.label(format!("PID: {}", child.id()));
                }
                if let Some(message) = &self.status_message {
                    ui.colored_label(message.color, &message.text);
                }
            });

            ui.add_space(SECTION_SPACING);

            ui.horizontal(|ui| {
                if ui.button("\u{1F504} Restart Preview").clicked() {
                    self.restart_daemon();
                }
                if ui.button("\u{2796} Hide to Tray").clicked() {
                    self.hide_window(ctx);
                }
            });

            ui.add_space(SECTION_SPACING);
            ui.separator();
            ui.add_space(SECTION_SPACING);

            ui.group(|ui| {
                ui.label(egui::RichText::new("Tips").strong());
                ui.label("• Tab/Shift+Tab: Cycle characters");
                ui.label("• Right-click drag: Move thumbnails");
                ui.label("• Left-click: Focus EVE window");
                ui.label("• Tray icon: Show/hide manager");
            });
        });

        ctx.request_repaint_after(Duration::from_millis(DAEMON_CHECK_INTERVAL_MS));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Err(err) = self.stop_daemon() {
            error!(error = ?err, "Failed to stop daemon during shutdown");
        }
        info!("Manager exiting");
    }
}

fn spawn_preview_daemon() -> Result<Child> {
    let exe_path = std::env::current_exe().context("Failed to resolve executable path")?;
    Command::new(exe_path)
        .arg("--preview")
        .spawn()
        .context("Failed to spawn preview daemon")
}

fn create_tray_icon() -> Result<(Option<TrayItem>, Receiver<TrayCommand>)> {
    let (tx, rx) = mpsc::sync_channel::<TrayCommand>(4);

    let (data, width, height) = load_tray_icon_pixels()?;
    let mut tray = TrayItem::new(
        "EVE-L Preview",
        IconSource::Data {
            data,
            width: width as i32,
            height: height as i32,
        },
    )?;

    info!("Tray icon created");

    let toggle_tx = tx.clone();
    tray.add_menu_item("Show/Hide Manager", move || {
        let _ = toggle_tx.send(TrayCommand::ToggleWindow);
    })?;

    tray.inner_mut().add_separator().ok();

    let quit_tx = tx;
    tray.add_menu_item("Quit", move || {
        let _ = quit_tx.send(TrayCommand::Quit);
    })?;

    Ok((Some(tray), rx))
}

fn load_tray_icon_pixels() -> Result<(Vec<u8>, u32, u32)> {
    let icon_bytes = include_bytes!("../../assets/tray-icon.png");
    let decoder = png::Decoder::new(Cursor::new(icon_bytes));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    let raw = &buf[..info.buffer_size()];

    let pixels = match info.color_type {
        png::ColorType::Rgba => convert_rgba_to_argb(raw),
        png::ColorType::Rgb => convert_rgb_to_argb(raw),
        other => {
            return Err(anyhow!(
                "Unsupported tray icon color type {:?} (expected RGB or RGBA)",
                other
            ))
        }
    };

    Ok((pixels, info.width, info.height))
}

fn convert_rgba_to_argb(raw: &[u8]) -> Vec<u8> {
    let mut argb = Vec::with_capacity(raw.len());
    for chunk in raw.chunks_exact(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];
        argb.extend_from_slice(&[a, r, g, b]);
    }
    argb
}

fn convert_rgb_to_argb(raw: &[u8]) -> Vec<u8> {
    let mut argb = Vec::with_capacity(raw.len() / 3 * 4);
    for chunk in raw.chunks_exact(3) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        argb.extend_from_slice(&[0xFF, r, g, b]);
    }
    argb
}

pub fn run_gui() -> Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
            .with_title("EVE-L Preview Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "EVE-L Preview Manager",
        options,
        Box::new(|cc| Ok(Box::new(ManagerApp::new(cc)))),
    )
    .map_err(|err| anyhow!("Failed to launch egui manager: {err}"))
}
