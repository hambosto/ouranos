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
      image.path = "~/wallpapers/wallpaper.png";

      transition = {
        transition_type = "fade";
        duration = 1.5;
        edge_smoothness = 0.3;
        transition_color = "#000000";

        wipe.direction = 0.0;

        disc = {
          center_x = 0.5;
          center_y = 0.5;
        };

        stripes = {
          stripe_count = 12.0;
          angle = 30.0;
        };

        honeycomb = {
          cell_size = 0.04;
          center_x = 0.5;
          center_y = 0.5;
        };
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

Home Manager takes care of the config file, systemd service, and restart triggers.

### From source

```
cargo build --release
target/release/ouranos
```

### Manual

Copy the binary somewhere on your PATH, then drop a config at `~/.config/ouranos/config.toml`:

```toml
[image]
path = "/path/to/wallpaper.png"
```

Run it however you want.

## Config

Location: `$XDG_CONFIG_HOME/ouranos/config.toml`

Only `[image]` is required. Everything else has sensible defaults.

### `[image]`

| Field | Type | Description |
|-------|------|-------------|
| `path` | string | Path to the image. PNG, JPEG, WebP, GIF, BMP. |

### `[transition]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `transition_type` | string | `fade` | Effect to use |
| `duration` | float | `1.5` | Seconds |
| `edge_smoothness` | float | `0.3` | Transition edge softness |
| `transition_color` | hex | `#000000` | Start color (`#RGB` or `#RRGGBBAA`) |

Available effects: `none` (instant snap), `fade`, `wipe`, `disc`, `stripes`, `zoom`, `honeycomb`

### `[transition.wipe]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | float | `0.0` | 0=right, 1=left, 2=up, 3=down. |

### `[transition.disc]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `center_x` | float | `0.5` | Horizontal origin (0.0–1.0). |
| `center_y` | float | `0.5` | Vertical origin (0.0–1.0). |

### `[transition.stripes]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stripe_count` | float | `12.0` | Number of stripes. |
| `angle` | float | `30.0` | Degrees. |

### `[transition.honeycomb]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cell_size` | float | `0.04` | Hex cell size. |
| `center_x` | float | `0.5` | Horizontal origin. |
| `center_y` | float | `0.5` | Vertical origin. |

### `[resize]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | string | `crop` | `no`, `crop`, `fit`, `stretch` |
| `crop_gravity` | string | `center` | `top-left`, `top`, `top-right`, `left`, `center`, `right`, `bottom-left`, `bottom`, `bottom-right` |
| `fill_color` | [u8; 4] | `[0, 0, 0, 255]` | RGBA for letterboxing |
| `filter` | string | `lanczos3` | `nearest`, `bilinear`, `catmull-rom`, `mitchell`, `lanczos3` |

## How it works

Reads config, connects to Wayland, creates a layer surface per output, renders the image into SHM buffers, commits it. If a transition is configured, it animates from the start color to the target image using calloop frame callbacks.

```
main.rs
  |
  +-- Config::load()                  config.rs
  |     reads $XDG_CONFIG_HOME/ouranos/config.toml
  |
  +-- wayland::run(config)            wayland/mod.rs
        |  connects, binds globals (compositor, layer-shell, shm, output),
        |  creates calloop EventLoop + WaylandSource, runs it
        |
        +-- handlers.rs               protocol glue (SCTK traits)
        |     on new_output           -> State::create_surfaces()
        |     on configure            -> Surface::configure() + render_or_warn()
        |     on frame                -> Surface::tick()
        |     on scale_factor_changed -> Surface::rescale() + render_or_warn()
        |     on closed / output_destroyed -> drop surfaces
        |
        +-- state.rs                  State: outputs, surfaces, config
              render_pending()
                Image::open()               image/mod.rs (decode + EXIF orient)
                resize::apply()             image/resize.rs (no/crop/fit/stretch)
                Surface::start_transition() wayland/surface.rs
                  Animation { Transition, SHM SlotPool, pixels }
                  present() per frame: tick transition, attach SHM buffer,
                  damage, commit
```

## Multi-monitor

Creates one layer surface per connected output. Handles hot-plug: surfaces are created when monitors connect and destroyed when they disconnect. Each surface gets its own independently resized image and transition.

## Environment

| Variable | Required | Default |
|----------|----------|---------|
| `WAYLAND_DISPLAY` | Yes | — |
| `XDG_CONFIG_HOME` | No | `~/.config` |

## Troubleshooting

- **WAYLAND_DISPLAY not set** — Not in a Wayland session.
- **failed to find config file** — File missing or bad permissions.
- **path is not a file** — Image path points to nothing.
- **failed to detect image format** — Not a supported image type.
- **no surfaces were configured by the compositor** — No outputs seen.
- **Wallpaper doesn't show** — Compositor might not support layer shell, or another wallpaper process is running.

## License

[MIT](LICENSE)
