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
cargo install --git https://github.com/humbertogfs55/dotafetch --locked

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
# exists but isn't on PATH anywhere. Add it to the current shell's rc file
# so new terminals pick it up automatically, rather than leaving the user
# with a binary they can't run.
case "$(basename "${SHELL:-}")" in
    fish)
        rc="$HOME/.config/fish/config.fish"
        line="fish_add_path $CARGO_BIN"
        ;;
    zsh)
        rc="$HOME/.zshrc"
        line="export PATH=\"$CARGO_BIN:\$PATH\""
        ;;
    *)
        rc="$HOME/.bashrc"
        line="export PATH=\"$CARGO_BIN:\$PATH\""
        ;;
esac

mkdir -p "$(dirname "$rc")"
if ! grep -qF "$CARGO_BIN" "$rc" 2>/dev/null; then
    printf '\n# added by dotafetch install.sh\n%s\n' "$line" >> "$rc"
fi

echo
echo "Installed. $CARGO_BIN wasn't on your PATH, so it's been added to $rc."
echo "Open a new terminal (or 'source $rc') and run 'dotafetch'."
