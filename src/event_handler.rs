use anyhow::Result;
use std::collections::HashMap;
use x11rb::connection::Connection;
use x11rb::protocol::damage::ConnectionExt as DamageExt;
use x11rb::protocol::Event::{self, CreateNotify, DamageNotify, DestroyNotify, PropertyNotify};
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as WrapperExt;

use crate::config::Config;
use crate::thumbnail::Thumbnail;
use crate::x11_utils::{is_window_eve, CachedAtoms};

pub fn handle_event<'a>(
    conn: &'a RustConnection,
    screen: &Screen,
    config: &'a Config,
    eves: &mut HashMap<Window, Thumbnail<'a>>,
    event: Event,
    atoms: &CachedAtoms,
    check_and_create_window: impl Fn(&'a RustConnection, &Screen, &'a Config, Window, &CachedAtoms) -> Result<Option<Thumbnail<'a>>>,
) -> Result<()> {
    match event {
        DamageNotify(event) => {
            if let Some(thumbnail) = eves
                .values()
                .find(|thumbnail| thumbnail.damage == event.damage)
            {
                thumbnail.update()?; // TODO: add fps limiter?
                conn.damage_subtract(event.damage, 0u32, 0u32)?;
                conn.flush()?;
            }
        }
        CreateNotify(event) => {
            if let Some(thumbnail) = check_and_create_window(conn, screen, config, event.window, atoms)? {
                eves.insert(event.window, thumbnail);
            }
        }
        DestroyNotify(event) => {
            eves.remove(&event.window);
        }
        PropertyNotify(event) => {
            if event.atom == atoms.wm_name
                && let Some(thumbnail) = eves.get_mut(&event.window)
                && let Some(character_name) = is_window_eve(conn, event.window, atoms)?
            {
                thumbnail.character_name = character_name;
                thumbnail.update_name()?;
            } else if event.atom == atoms.wm_name
                && let Some(thumbnail) = check_and_create_window(conn, screen, config, event.window, atoms)?
            {
                eves.insert(event.window, thumbnail);
            } else if event.atom == atoms.net_wm_state
                && let Some(thumbnail) = eves.get_mut(&event.window)
                && let Some(state) = conn
                    .get_property(false, event.window, event.atom, AtomEnum::ATOM, 0, 1024)?
                    .reply()?
                    .value32()
                && state.collect::<Vec<_>>().contains(&atoms.net_wm_state_hidden)
            {
                thumbnail.minimized()?;
            }
        }
        Event::FocusIn(event) => {
            if let Some(thumbnail) = eves.get_mut(&event.event) {
                thumbnail.minimized = false;
                thumbnail.focused = true;
                thumbnail.border(true)?;
                if config.hide_when_no_focus && eves.values().any(|x| !x.visible) {
                    for thumbnail in eves.values_mut() {
                        thumbnail.visibility(true)?;
                    }
                }
            }
        }
        Event::FocusOut(event) => {
            if let Some(thumbnail) = eves.get_mut(&event.event) {
                thumbnail.focused = false;
                thumbnail.border(false)?;
                if config.hide_when_no_focus && eves.values().all(|x| !x.focused && !x.minimized) {
                    for thumbnail in eves.values_mut() {
                        thumbnail.visibility(false)?;
                    }
                }
            }
        }
        Event::ButtonPress(event) => {
            if let Some((_, thumbnail)) = eves
                .iter_mut()
                .find(|(_, thumb)| thumb.is_hovered(event.root_x, event.root_y) && thumb.visible)
            {
                let geom = conn.get_geometry(thumbnail.window)?.reply()?;
                thumbnail.input_state.drag_start = (event.root_x, event.root_y);
                thumbnail.input_state.win_start = (geom.x, geom.y);
                thumbnail.input_state.dragging = true;
            }
        }
        Event::ButtonRelease(event) => {
            if let Some((_, thumbnail)) = eves
                .iter_mut()
                .find(|(_, thumb)| thumb.is_hovered(event.root_x, event.root_y) && thumb.input_state.dragging)
            {
                if event.detail == 1 //
                    && thumbnail.input_state.drag_start == (event.root_x, event.root_y)
                {
                    thumbnail.focus()?;
                }
                thumbnail.input_state.dragging = false;
            }
        }
        Event::MotionNotify(event) => {
            if let Some((_, thumbnail)) = eves.iter_mut().find(|(_, thumb)| {
                thumb.input_state.dragging && thumb.is_hovered(event.root_x, event.root_y)
            }) {
                // TODO: snap to be inline with other thumbnails
                let dx = event.root_x - thumbnail.input_state.drag_start.0;
                let dy = event.root_y - thumbnail.input_state.drag_start.1;
                let new_x = thumbnail.input_state.win_start.0 + dx;
                let new_y = thumbnail.input_state.win_start.1 + dy;
                thumbnail.reposition(new_x, new_y)?;
            }
        }
        _ => (),
    }
    Ok(())
}
