#!/bin/sh
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
mode="install"
prefix=""
yes=0
menu=1

usage() {
    cat <<'USAGE'
usage: install.sh [--user | --system | --prefix DIR] [--no-menu] [--yes] [--uninstall]

  --user       install for this user under ~/.local
  --system     install for all users under /usr/local, needs root
  --prefix     install under DIR
  --no-menu    skip the application menu entry
  --yes        do not ask questions
  --uninstall  remove an earlier install from the chosen place
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --user) prefix="$HOME/.local" ;;
        --system) prefix="/usr/local" ;;
        --prefix)
            shift
            prefix="${1:?--prefix needs a directory}"
            ;;
        --no-menu) menu=0 ;;
        --yes) yes=1 ;;
        --uninstall) mode="uninstall" ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage >&2
            exit 2
            ;;
    esac
    shift
done

ask() {
    if [ "$yes" -eq 1 ]; then
        printf '%s\n' "$2"
        return
    fi
    printf '%s [%s] ' "$1" "$2" >&2
    read -r reply || reply=""
    if [ -z "$reply" ]; then
        printf '%s\n' "$2"
    else
        printf '%s\n' "$reply"
    fi
}

printf 'OOTRR installer\n\n'
printf 'OOTRR contains no game files. It only reads files you extracted yourself.\n\n'

if [ -z "$prefix" ]; then
    printf '1) Install for this user, under %s/.local\n' "$HOME"
    printf '2) Install for all users, under /usr/local, needs root\n\n'
    choice="$(ask 'Choose 1 or 2' '1')"
    case "$choice" in
        2) prefix="/usr/local" ;;
        *) prefix="$HOME/.local" ;;
    esac
fi

bin_dir="$prefix/bin"
share="$prefix/share"
entry="$share/applications/ootrr.desktop"
doc="$share/doc/ootrr"

if [ "$mode" = "uninstall" ]; then
    rm -f "$bin_dir/ootrr" "$entry"
    rm -rf "$doc"
    printf 'Removed OOTRR from %s\n' "$prefix"
    exit 0
fi

if [ "$menu" -eq 1 ] && [ "$yes" -eq 0 ]; then
    answer="$(ask 'Add OOTRR to the application menu? (y/n)' 'y')"
    case "$answer" in
        n|N|no|No) menu=0 ;;
    esac
fi

if [ ! -f "$here/ootrr" ]; then
    echo "ootrr was not found next to install.sh" >&2
    exit 1
fi

mkdir -p "$bin_dir" "$doc"
install -m 755 "$here/ootrr" "$bin_dir/ootrr"
for f in LICENSE NOTICE README.md; do
    if [ -f "$here/$f" ]; then
        install -m 644 "$here/$f" "$doc/$f"
    fi
done

if [ "$menu" -eq 1 ]; then
    mkdir -p "$share/applications"
    sed "s|@BIN@|$bin_dir/ootrr|g" "$here/ootrr.desktop" > "$entry"
    chmod 644 "$entry"
fi

printf '\nInstalled to %s\n' "$bin_dir/ootrr"
case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) printf 'Add %s to your PATH to run ootrr from any terminal.\n' "$bin_dir" ;;
esac
printf 'Run: ootrr serve\n'
