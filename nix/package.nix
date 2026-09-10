{
  date,
  lib,
  libxkbcommon,
  pkg-config,
  rev ? "unknown",
  rustPlatform,
  version ? "git",
}:
rustPlatform.buildRustPackage (final: {
  pname = "ouranos";
  inherit version;

  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;
  doCheck = false;

  buildInputs = [ libxkbcommon ];
  nativeBuildInputs = [ pkg-config ];

  OURANOS_BUILD_VERSION = "unstable ${date} (commit ${rev})";

  meta = {
    description = "A Wayland wallpaper daemon with animated transitions.";
    homepage = "https://github.com/hambosto/ouranos";
    license = lib.licenses.mit;
    mainProgram = "ouranos";
    platforms = lib.platforms.linux;
  };
})
