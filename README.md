# Phylax

A Polkit authentication agent written in GTK4 for Arch Linux.

> Forked from [Soteria](https://github.com/imvaskel/soteria) by Vaskel.

![Example authentication popup](.github/example_popup.png)

## Installation

### Manual

Requires Rust >= 1.85, GTK4 development headers, and Polkit.

```bash
git clone https://github.com/jakeb-grant/phylax
cd phylax
cargo install --locked --path .
```

## Configuration

If your `polkit-agent-helper-1` is in a non-standard location, create `~/.config/phylax/config.toml`:

```toml
helper_path = "/path/to/your/helper"
```

Optional custom styling via `~/.config/phylax/style.css`.

## Usage

Add to your compositor startup. For Hyprland:

```conf
exec-once = /path/to/phylax
windowrulev2=pin,class:io.github.jakebgrant.phylax
```

## Debugging

```bash
RUST_LOG=debug phylax
```

## License

Apache-2.0
