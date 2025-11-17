{
  pkgs ? import <nixpkgs> { },
}:
let
  manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = "${manifest.name}-test";
  version = manifest.version;

  cargoLock.lockFile = ./Cargo.lock;

  src = pkgs.lib.cleanSource ./.;

  # Run tests instead of building
  buildPhase = ''
    cargo test --no-fail-fast 2>&1 | tee test-output.txt
  '';

  installPhase = ''
    mkdir -p $out
    cp test-output.txt $out/ || true
    echo "Tests completed"
  '';

  nativeBuildInputs = with pkgs; [ pkg-config ];
  buildInputs = with pkgs; [
    xorg.libX11
    xorg.libXfixes
    xorg.libXdamage
    gtk3
    libappindicator
  ];
}
