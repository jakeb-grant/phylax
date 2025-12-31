# Phylax

A Polkit authentication agent written in GTK4 for Arch Linux.

![Rust](https://img.shields.io/badge/rust-1.85%2B-orange)
![License](https://img.shields.io/badge/license-Apache--2.0-blue)
![Arch Linux](https://img.shields.io/badge/Arch-Linux-1793D1?logo=arch-linux)

> Forked from [Soteria](https://github.com/imvaskel/soteria) by Vaskel.

![Example authentication popup](.github/example_popup.png)

## Features

- **GTK4 Native** - Modern, lightweight authentication dialogs
- **User Selection** - Switch between available identities when authenticating
- **Retry Support** - Clear feedback on failed attempts with retry capability
- **Custom Styling** - Optional CSS theming support
- **Secure** - Passwords are zeroized from memory after use

## Installation

### AUR (recommended)

```bash
yay -S phylax-git
```

### Prebuilt binary

Download from [GitHub Releases](https://github.com/jakeb-grant/phylax/releases):

```bash
tar -xzf phylax-v*.tar.gz
sudo mv phylax /usr/local/bin/
```

### From source

```bash
git clone https://github.com/jakeb-grant/phylax.git
cd phylax
cargo build --release
sudo cp target/release/phylax /usr/local/bin/
```

### Dependencies

- GTK4
- Polkit

## Usage

Add Phylax to your compositor startup. For Hyprland:

```conf
exec-once = phylax
windowrulev2 = pin, class:io.github.jakebgrant.phylax
```

For Sway:

```conf
exec phylax
```

### Debugging

```bash
RUST_LOG=debug phylax
```

## Configuration

Configuration files are stored in `~/.config/phylax/`.

### config.toml

```toml
# Custom polkit helper path (only needed for non-standard locations)
helper_path = "/usr/lib/polkit-1/polkit-agent-helper-1"
```

### style.css

Custom GTK4 styling for the authentication dialog:

```css
window {
    background-color: #1e1e2e;
}

button {
    border-radius: 8px;
}
```

## License

Apache-2.0
