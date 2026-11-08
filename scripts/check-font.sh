#!/usr/bin/env bash
# Font dependency check for useRA.
#
# useRA renders Nerd Font glyphs (private-use-area code points). If the
# active terminal font is not a Nerd Font, icons show as tofu/boxes.
# This script reports what to install on Debian 12/13; it never installs
# anything on its own (no silent system changes — AGENTS.md §6).

set -u

have_fc() { command -v fc-list >/dev/null 2>&1; }

check_nerd_font() {
    if ! have_fc; then
        echo "UNKNOWN: fontconfig (fc-list) not available"
        return
    fi
    if fc-list | grep -qiE 'nerd ?font|nerdfont|NF\b|JetBrains.*NL|SymbolsNerdFont'; then
        echo "OK: a Nerd Font is installed"
    else
        echo "MISSING: no Nerd Font found"
    fi
}

suggest() {
    cat <<'EOF'
Debian does not ship Nerd Fonts in its repos. Install one of:

  1. Official installer (recommended):
       curl -fsSLO https://github.com/ryanoasis/nerd-fonts/raw/master/patched-fonts/JetBrainsMono/Ligatures/JetBrainsMonoNerdFont-Regular.ttf
       mkdir -p ~/.local/share/fonts && mv JetBrainsMonoNerdFont-Regular.ttf ~/.local/share/fonts/
       fc-cache -f ~/.local/share/fonts
     Then select the font in your terminal emulator (e.g. "JetBrainsMono Nerd Font").

  2. Full family via the Nerd Fonts install script:
       curl -fsS https://raw.githubusercontent.com/ryanoasis/nerd-fonts/master/install.sh | bash -s -- JetBrainsMono
     (review the script before piping — it writes to your font dirs)

  3. Manual download of any patched font from https://www.nerdfonts.com/
EOF
}

check_nerd_font
if ! fc-list 2>/dev/null | grep -qiE 'nerd ?font|nerdfont|NF\b|JetBrains.*NL|SymbolsNerdFont'; then
    suggest
    exit 1
fi
