# useRA — Installation Guide

Target platforms: **Debian 12 (bookworm)** and **Debian 13 (trixie)**,
x86-64. Other distributions may work but are untested.

## Option A — Build from source

### Prerequisites

| Requirement | Debian 12 | Debian 13 |
| --- | --- | --- |
| Rust toolchain ≥ 1.88 | via [rustup](https://rustup.rs) (repo `rustc` is 1.63 — too old) | via [rustup](https://rustup.rs) (repo `rustc` is 1.85 — too old) |
| C compiler + linker | `sudo apt install build-essential` | same |
| Terminal with UTF-8 | default | default |
| Nerd Font (icons) | see [Nerd Font](#nerd-font-requirement) | same |

### Build

```sh
git clone <repo-url> usera && cd usera
cargo build --release
./target/release/usera          # read-only as normal user
sudo ./target/release/usera     # full user/group management
```

The release binary is statically stripped (~620 KiB) and links glibc
dynamically — it runs on Debian 12 (glibc ≥ 2.34 symbols only; newer
glibc features are weakly referenced and used only when present).

### Install the binary system-wide

```sh
sudo install -m 755 target/release/usera /usr/local/bin/usera
```

## Option B — One-liner install (release binary)

Once the repository is public on GitHub under `munMisbah/useRA` with release
artifacts attached, users install like Superfile:

```sh
bash -c "$(curl -sLo- https://raw.githubusercontent.com/munMisbah/useRA/main/install.sh)"
```

The script downloads the latest `usera-linux-x86_64.tar.gz` from GitHub
Releases, verifies its sha256 checksum, and installs to
`/usr/local/bin` (sudo if needed) or `~/.local/bin` without root.

See [docs/INSTALL.md](docs/INSTALL.md) for manual installation,
prerequisites, and the Nerd Font setup.

## Nerd Font requirement

The interface uses Nerd Font glyphs for icons. Without one, icons render
as boxes — the app still works. Verify with:

```sh
scripts/check-font.sh
```

Debian 12/13 repositories do not package Nerd Fonts. Quick install
(user-local, no sudo needed):

```sh
curl -fsSLO https://github.com/ryanoasis/nerd-fonts/raw/master/patched-fonts/JetBrainsMono/Ligatures/JetBrainsMonoNerdFont-Regular.ttf
mkdir -p ~/.local/share/fonts && mv JetBrainsMonoNerdFont-Regular.ttf ~/.local/share/fonts/
fc-cache -f ~/.local/share/fonts
```

Then select **JetBrainsMono Nerd Font** in your terminal emulator's
appearance settings.

## Running

```sh
usera
```

- Without root: full read-only inspection (users, groups, sessions).
- With `sudo`: adds user/group management operations (see README
  shortcuts tables). Write operations are refused without root.
- Audit log: `/var/log/usera/audit.log` (root) or
  `$XDG_STATE_HOME/usera/audit.log`.

## Updating

To update `useRA` to the latest release:

```sh
usera update
# or with sudo if installed in /usr/local/bin:
sudo usera update
```

Alternatively, re-run the one-liner installer:

```sh
bash -c "$(curl -fsSL https://raw.githubusercontent.com/munMisbah/useRA/main/install.sh)"
```

## Uninstall

```sh
sudo rm /usr/local/bin/usera
rm -rf ~/.local/state/usera        # user audit log
sudo rm -rf /var/log/usera         # root audit log, if present
```
