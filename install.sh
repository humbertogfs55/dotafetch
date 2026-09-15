#!/usr/bin/env bash
# Installs dotafetch straight from GitHub via cargo - no prebuilt binaries,
# no release assets, just `cargo install --git`. Usage:
#   curl -fsSL https://raw.githubusercontent.com/humbertogfs55/dotafetch/main/install.sh | bash
set -euo pipefail

command -v cargo >/dev/null 2>&1 || {
    echo "dotafetch: cargo (Rust) is required - install it from https://rustup.rs and re-run" >&2
    exit 1
}

echo "Installing dotafetch (cargo install --git)..."
# --force: without it, cargo silently no-ops on rerun once already
# installed, so re-running this script would never pick up new commits.
cargo install --git https://github.com/humbertogfs55/dotafetch --locked --force

CARGO_BIN="$HOME/.cargo/bin"

# Happy path (cargo came from rustup): $CARGO_BIN is already on PATH, so
# `dotafetch` just works in any terminal, no further action needed.
case ":$PATH:" in
    *":$CARGO_BIN:"*)
        echo
        echo "Installed. Run 'dotafetch'."
        exit 0
        ;;
esac

# Edge case (cargo came from a distro package, not rustup): $CARGO_BIN
# exists but isn't on PATH anywhere. Add it to every shell rc file that
# actually exists on disk, rather than guessing from $SHELL - that env var
# is the login shell recorded at session start and can be stale (e.g. zsh
# launched from .bashrc without updating the passwd entry), so it doesn't
# reliably say which rc file the next terminal will source.
touched=()
add_path_line() {
    mkdir -p "$(dirname "$1")"
    if ! grep -qF "$CARGO_BIN" "$1" 2>/dev/null; then
        printf '\n# added by dotafetch install.sh\n%s\n' "$2" >> "$1"
    fi
    touched+=("$1")
}
[ -f "$HOME/.bashrc" ] && add_path_line "$HOME/.bashrc" "export PATH=\"$CARGO_BIN:\$PATH\""
[ -f "$HOME/.zshrc" ] && add_path_line "$HOME/.zshrc" "export PATH=\"$CARGO_BIN:\$PATH\""
[ -f "$HOME/.config/fish/config.fish" ] && add_path_line "$HOME/.config/fish/config.fish" "fish_add_path $CARGO_BIN"

# No known rc file exists yet (fresh account) - fall back to creating one
# for the shell that's actually about to read it.
if [ ${#touched[@]} -eq 0 ]; then
    case "$(basename "${SHELL:-}")" in
        fish) add_path_line "$HOME/.config/fish/config.fish" "fish_add_path $CARGO_BIN" ;;
        zsh) add_path_line "$HOME/.zshrc" "export PATH=\"$CARGO_BIN:\$PATH\"" ;;
        *) add_path_line "$HOME/.bashrc" "export PATH=\"$CARGO_BIN:\$PATH\"" ;;
    esac
fi

echo
echo "Installed. $CARGO_BIN wasn't on your PATH, so it's been added to:"
printf '  %s\n' "${touched[@]}"
echo "Open a new terminal and run 'dotafetch'."
