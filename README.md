<div align="center">

# ouranos

A Wayland wallpaper daemon with animated transitions.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![Nix Flake](https://img.shields.io/badge/nix-flake-purple.svg)](https://nixos.org/)
[![Wayland](https://img.shields.io/badge/wayland-layer--shell-green.svg)](https://wayland.app/)
[![Linux](https://img.shields.io/badge/platform-linux-lightgrey.svg)](https://www.linux.org/)

</div>

## Why

Everything on my NixOS setup is declarative. Window manager, dotfiles, keybindings — all in config. Wallpaper was the one thing that didn't fit.

After switching to Niri I didn't find a tool that worked the way I wanted, so I wrote one. It does three things:

- Sets wallpaper on login
- Configures through Home Manager
- Restarts when the config changes

That's it.

## Requirements

- Wayland compositor with layer shell (Niri, Hyprland, Sway, etc.)
- `WAYLAND_DISPLAY` set
- Rust 1.87+ (2024 edition)

## Install

### Nix

```nix
{
  inputs.ouranos.url = "github:hambosto/ouranos";
}
```

```nix
{
  imports = [ inputs.ouranos.homeManagerModules.default ];

  services.ouranos = {
    enable = true;
    settings = {
      image = "/absolute/path/to/wallpaper.png";

      transition = {
        transition_type = "fade";
        duration = 1.5;
        edge_smoothness = 0.3;
        transition_color = "#000000";
      };

      resize = {
        strategy = "crop";
        crop_gravity = "center";
        fill_color = [0 0 0 255];
        filter = "lanczos3";
      };
    };
  };
}
```

Home Manager writes the config file, enables the `ouranos` user service under the Wayland session target, and restarts it whenever the settings change.

### From source

Needs `libxkbcommon` headers to link (e.g. `libxkbcommon-dev` on Debian/Ubuntu).

```
cargo build --release
target/release/ouranos
```

### Manual

Copy the binary somewhere on your PATH, then drop a config at `~/.config/ouranos/config.toml`:

```toml
image = "/path/to/wallpaper.png"
```

Run it however you want.

## Config

Location: `$XDG_CONFIG_HOME/ouranos/config.toml` (falls back to `$XDG_CONFIG_DIRS`).

Only `image` is required. The `[transition]` and `[resize]` tables each fall back to their defaults when omitted entirely, but TOML tables are all-or-nothing: if you include a table, you must set every field in it, otherwise parsing fails with a missing-field error.

### `image`

Absolute path to the image. Anything the `image` crate can decode: PNG, JPEG, WebP, GIF, BMP, TIFF, and more. EXIF orientation is honored.

### `[transition]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `transition_type` | string | `fade` | Effect to use |
| `duration` | float | `1.5` | Seconds; non-finite or negative values fall back to `1.5` |
| `edge_smoothness` | float | `0.3` | Edge softness in `0.0..=1.0` (clamped; non-finite falls back to `0.3`) |
| `transition_color` | hex | `#000000` | Start color; `#RGB`, `#RRGGBB`, `#RGBA`, or `#RRGGBBAA` (alpha is ignored, treated as opaque) |

Available effects: `none` (instant snap, ignores `duration`), `fade`, `wipe`, `disc`, `stripes`, `zoom`, `honeycomb`, `vortex`, `pixel`, `diamond`, `golden`, `melt`

Per-effect params (wipe direction, disc/honeycomb/vortex/pixel/diamond/golden origin, stripe angle/count, honeycomb/pixel/diamond cell size, melt angle) are randomized every run. `edge_smoothness` has no effect on `none`, `fade`, or `zoom`.

### `[resize]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | string | `crop` | `no`, `crop`, `fit`, `stretch` |
| `crop_gravity` | string | `center` | `top-left`, `top`, `top-right`, `left`, `center`, `right`, `bottom-left`, `bottom`, `bottom-right` |
| `fill_color` | [u8; 4] | `[0, 0, 0, 255]` | RGBA canvas fill for `no`/`fit` |
| `filter` | string | `lanczos3` | `nearest`, `bilinear`, `catmull-rom`, `mitchell`, `lanczos3` |

Strategies: `no` centers the image unscaled (cropping centered pixels if larger), `crop` scales to fill and crops the excess around `crop_gravity`, `fit` scales to fit and letterboxes with `fill_color`, `stretch` scales to the exact output size ignoring aspect ratio.

## How it works

Reads config, connects to Wayland, creates a background layer surface per output, renders the image into SHM buffers, and commits it. The transition animates from the start color to the target image, one frame per Wayland frame callback, until it finishes — then the surface idles without requesting further frames.

## Multi-monitor

Creates one layer surface per connected output. Handles hot-plug: surfaces are created when monitors connect and destroyed when they disconnect. Scale-factor and transform changes re-render the affected output. Each output gets its own independently resized image and transition.

## Environment

| Variable | Required | Default |
|----------|----------|---------|
| `WAYLAND_DISPLAY` | Yes | — |
| `XDG_CONFIG_HOME` | No | `~/.config` |

Logs go to stderr with file and line numbers; under Home Manager, inspect them with `journalctl --user -u ouranos`.

## Troubleshooting

- **failed to connect to wayland** — Not in a Wayland session (`WAYLAND_DISPLAY` missing or stale).
- **failed to find config file** — File missing or bad permissions.
- **cannot parse config / missing field** — Invalid TOML, or a `[transition]`/`[resize]` table with only some fields set (tables are all-or-nothing).
- **failed to open image** — Image path points to nothing or isn't readable.
- **failed to detect image format** — Not a supported image type.
- **zwlr_layer_shell_v1 / wl_shm / wl_compositor not available** — Compositor doesn't support layer shell.
- **no surfaces were configured by the compositor yet** — No outputs seen; usually resolves once the compositor announces one.
- **Wallpaper doesn't show** — Compositor might not support layer shell, or another wallpaper process is running.

## License

[MIT](LICENSE)
