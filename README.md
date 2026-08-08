# ouranos

A wallpaper daemon for Wayland. Sets an image on all connected monitors with optional animated transitions.

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
- Rust 1.85+ (2024 edition)

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
| `path` | string | Path to the image. PNG, JPEG, WebP. |

### `[transition]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `transition_type` | string | `fade` | Effect to use |
| `duration` | float | `1.5` | Seconds |
| `edge_smoothness` | float | `0.3` | Transition edge softness |
| `transition_color` | hex | `#000000` | Start color (`#RGB` or `#RRGGBBAA`) |

Available effects: `none`, `simple`, `fade`, `wipe`, `disc`, `stripes`, `zoom`, `honeycomb`

### `[transition.wipe]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `direction` | float | `0.0` | 0–3, randomized when 0 |

### `[transition.disc]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `center_x` | float | `0.5` | Horizontal origin (0.0–1.0), randomized when 0.5 |
| `center_y` | float | `0.5` | Vertical origin (0.0–1.0), randomized when 0.5 |

### `[transition.stripes]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stripe_count` | float | `12.0` | Number of stripes, randomized when 12 |
| `angle` | float | `30.0` | Degrees, randomized when 30 |

### `[transition.honeycomb]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cell_size` | float | `0.04` | Hex cell size, randomized when 0.04 |
| `center_x` | float | `0.5` | Horizontal origin, randomized when 0.5 |
| `center_y` | float | `0.5` | Vertical origin, randomized when 0.5 |

### `[resize]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | string | `crop` | `no`, `crop`, `fit`, `stretch` |
| `crop_gravity` | string | `center` | `top-left`, `top`, `top-right`, `left`, `center`, `right`, `bottom-left`, `bottom`, `bottom-right` |
| `fill_color` | [u8; 4] | `[0, 0, 0, 255]` | RGBA for letterboxing |
| `filter` | string | `lanczos3` | `nearest`, `bilinear`, `catmull-rom`, `mitchell`, `lanczos3` |

## How it works

Reads config, connects to Wayland, creates a layer surface per output, renders the image into SHM buffers, commits it. If a transition is configured, it animates from the start color to the target image using calloop frame callbacks.

## Troubleshooting

- **WAYLAND_DISPLAY not set** — Not in a Wayland session.
- **Failed to read config** — File missing or bad permissions.
- **path does not exist** — Image path points to nothing.
- **Failed to detect image format** — Not a supported image type.
- **Wallpaper doesn't show** — Compositor might not support layer shell, or another wallpaper process is running.

## Environment

| Variable | Required | Default |
|----------|----------|---------|
| `WAYLAND_DISPLAY` | Yes | — |
| `XDG_CONFIG_HOME` | No | `~/.config` |

## License

[MIT](LICENSE)
