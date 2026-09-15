#!/usr/bin/env bash
# Undoes install.sh: removes the dotafetch binary (via cargo, which already
# owns install/uninstall for anything installed with `cargo install`) and
# strips the PATH line install.sh may have added to shell rc files. Usage:
#   curl -fsSL https://raw.githubusercontent.com/humbertogfs55/dotafetch/main/uninstall.sh | bash
set -euo pipefail

if command -v cargo >/dev/null 2>&1; then
    cargo uninstall dotafetch 2>/dev/null || echo "dotafetch: not installed via cargo, skipping binary removal"
fi

# Only ever remove the exact 2 lines install.sh added (comment marker +
# the PATH line right after it) - never touch anything else in these files.
removed=()
for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.config/fish/config.fish"; do
    [ -f "$rc" ] || continue
    if grep -qF '# added by dotafetch install.sh' "$rc"; then
        sed -i '/# added by dotafetch install\.sh/,+1d' "$rc"
        removed+=("$rc")
    fi
done

if [ ${#removed[@]} -gt 0 ]; then
    echo "Removed the PATH line install.sh added from:"
    printf '  %s\n' "${removed[@]}"
fi

echo
echo "Uninstalled."
