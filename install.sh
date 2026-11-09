#!/usr/bin/env bash
# useRA installer — the one-liner:
#
#   bash -c "$(curl -sLo- https://raw.githubusercontent.com/munMisbah/useRA/main/install.sh)"
#
# Downloads the latest release binary from GitHub Releases, verifies its
# checksum, and installs it. Set USERA_REPO to override the slug:
#
#   USERA_REPO=yourname/useRA bash -c "$(curl ...)"
#
# Review before running: https://github.com/munMisbah/useRA/blob/main/install.sh

set -euo pipefail

REPO="${USERA_REPO:-munMisbah/useRA}"
INSTALL_BIN="/usr/local/bin/usera"
STATE_DIR="$HOME/.local/state/usera"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

log()  { printf '\033[1;35museRA\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31museRA install failed:\033[0m %s\n' "$*" >&2; exit 1; }

# --- Platform check ----------------------------------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"
[ "$OS" = "Linux" ] || fail "unsupported OS '$OS' — useRA targets Linux"
[ "$ARCH" = "x86_64" ] || fail "unsupported architecture '$ARCH' — x86_64 only for now"

BASE_URL="https://github.com/${REPO}/releases/latest/download"

# --- Dependencies -------------------------------------------------------
command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 \
    || fail "curl or wget is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$2" "$1" || fail "download failed: $1"
    else
        wget -qO "$2" "$1" || fail "download failed: $1"
    fi
}

# --- Download -----------------------------------------------------------
log "fetching latest release from github.com/${REPO}"
fetch "${BASE_URL}/usera-linux-x86_64.tar.gz" "$TMP/usera.tar.gz"
fetch "${BASE_URL}/usera-linux-x86_64.tar.gz.sha256" "$TMP/usera.tar.gz.sha256"

# --- Verify -------------------------------------------------------------
EXPECTED="$(awk '{print $1}' "$TMP/usera.tar.gz.sha256")"
ACTUAL="$(sha256sum "$TMP/usera.tar.gz" | awk '{print $1}')"
[ "$EXPECTED" = "$ACTUAL" ] || fail "checksum mismatch — download corrupted or tampered"
log "checksum verified ($ACTUAL)"

tar -xzf "$TMP/usera.tar.gz" -C "$TMP"
[ -f "$TMP/usera" ] || fail "archive did not contain the 'usera' binary"

# --- Install ------------------------------------------------------------
PREV_VER=""
if command -v usera >/dev/null 2>&1; then
    PREV_VER="$(usera --version 2>/dev/null | awk '{print $2}' || true)"
fi

# Prefer /usr/local/bin (system-wide); fall back to ~/.local/bin without
# root. The binary itself needs root only when managing accounts.
if [ -w /usr/local/bin ] || [ "$(id -u)" = "0" ]; then
    install -m 755 "$TMP/usera" "$INSTALL_BIN"
    BIN_PATH="$INSTALL_BIN"
elif command -v sudo >/dev/null 2>&1; then
    log "requesting sudo to install to /usr/local/bin"
    sudo install -m 755 "$TMP/usera" "$INSTALL_BIN" \
        || fail "sudo install failed"
    BIN_PATH="$INSTALL_BIN"
else
    mkdir -p "$HOME/.local/bin"
    install -m 755 "$TMP/usera" "$HOME/.local/bin/usera"
    BIN_PATH="$HOME/.local/bin/usera"
    case ":$PATH:" in
        *":$HOME/.local/bin:"*) ;;
        *) log "NOTE: $HOME/.local/bin is not in your PATH — add it to your shell profile" ;;
    esac
fi

mkdir -p "$STATE_DIR"

if [ -n "$PREV_VER" ]; then
    NEW_VER="$("$BIN_PATH" --version 2>/dev/null | awk '{print $2}' || echo 'latest')"
    log "successfully updated useRA ($PREV_VER -> $NEW_VER) at $BIN_PATH"
else
    log "installed to $BIN_PATH"
fi
log ""
log "run it:            usera"
log "manage accounts:   sudo usera"
log "update in future:  usera update"
log "font check:        curl -sLo- ${BASE_URL%/*/*}/raw/main/scripts/check-font.sh | bash"
log "audit log:         ${STATE_DIR}/audit.log (root: /var/log/usera/audit.log)"
log ""
log "enjoy! https://github.com/${REPO}"
