# Soteria

A Polkit authentication agent written in GTK4 for Arch Linux.

![Example authentication popup](.github/example_popup.png)

## Installation

### AUR (Recommended)

```bash
yay -S soteria-git
# or
paru -S soteria-git
```

### Manual

Requires Rust >= 1.85, GTK4 development headers, and Polkit.

```bash
git clone https://github.com/imvaskel/soteria
cd soteria
cargo install --locked --path .
```

## Configuration

If your `polkit-agent-helper-1` is in a non-standard location, create `~/.config/soteria/config.toml`:

```toml
helper_path = "/path/to/your/helper"
```

Optional custom styling via `~/.config/soteria/style.css`.

## Usage

Add to your compositor startup. For Hyprland:

```conf
exec-once = /path/to/soteria
windowrulev2=pin,class:gay.vaskel.soteria
```

## Debugging

```bash
RUST_LOG=debug soteria
```

## License

Apache-2.0
