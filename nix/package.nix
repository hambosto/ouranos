{
  self,
  lib,
  pkg-config,
  rustPlatform,
  rust-jemalloc-sys,
  libxkbcommon,
}:
let
  fmtDate =
    raw:
    let
      year = builtins.substring 0 4 raw;
      month = builtins.substring 4 2 raw;
      day = builtins.substring 6 2 raw;
    in
    "${year}-${month}-${day}";
in
rustPlatform.buildRustPackage (final: {
  pname = "ouranos";
  version = "unstable-${fmtDate self.lastModifiedDate}-${self.shortRev or "dirty"}";

  src = lib.cleanSourceWith {
    filter =
      name: _:
      let
        baseName = baseNameOf (toString name);
      in
      !(lib.hasSuffix ".nix" baseName);
    src = lib.cleanSource ../.;
  };

  cargoLock.lockFile = ../Cargo.lock;

  doCheck = false;

  buildInputs = [
    libxkbcommon
    rust-jemalloc-sys
  ];

  nativeBuildInputs = [
    pkg-config
  ];

  WALLPAPER_BUILD_VERSION = "unstable ${fmtDate self.lastModifiedDate} (commit ${self.rev or "dirty"})";

  meta = {
    description = "A Wayland wallpaper daemon with animated transitions.";
    homepage = "https://github.com/hambosto/ouranos";
    license = lib.licenses.mit;
    mainProgram = "ouranos";
    platforms = lib.platforms.linux;
  };
})
