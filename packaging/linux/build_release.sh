#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
    echo "usage: build_release.sh <version> <binary> <out_dir>" >&2
    exit 2
fi

version="$1"
bin="$2"
out="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"

name="ootrr-$version-linux-x86_64"
tree="$work/$name"
mkdir -p "$tree"
install -m 755 "$bin" "$tree/ootrr"
install -m 755 "$here/install.sh" "$tree/install.sh"
install -m 644 "$here/ootrr.desktop" "$tree/ootrr.desktop"
for f in LICENSE NOTICE README.md; do
    install -m 644 "$repo/$f" "$tree/$f"
done
tar -C "$work" -czf "$out/OOTRR-$version-linux-x86_64.tar.gz" "$name"
echo "wrote $out/OOTRR-$version-linux-x86_64.tar.gz"

deb="$work/deb"
mkdir -p "$deb/DEBIAN" "$deb/usr/bin" "$deb/usr/share/applications" "$deb/usr/share/doc/ootrr"
install -m 755 "$bin" "$deb/usr/bin/ootrr"
sed "s|@BIN@|/usr/bin/ootrr|g" "$here/ootrr.desktop" > "$deb/usr/share/applications/ootrr.desktop"
chmod 644 "$deb/usr/share/applications/ootrr.desktop"
install -m 644 "$repo/LICENSE" "$deb/usr/share/doc/ootrr/copyright"
install -m 644 "$repo/NOTICE" "$deb/usr/share/doc/ootrr/NOTICE"
size="$(du -sk "$deb/usr" | cut -f1)"
cat > "$deb/DEBIAN/control" <<CONTROL
Package: ootrr
Version: $version
Section: utils
Priority: optional
Architecture: amd64
Installed-Size: $size
Maintainer: OOTRR contributors <noreply@users.noreply.github.com>
Homepage: https://github.com/realazk/ootrr
Description: Ocarina of Time remake randomizer
 OOTRR generates a seed and builds a mod folder from game files that you
 extracted yourself. It contains no game files. Run ootrr serve and open the
 link it prints.
CONTROL
dpkg-deb --build --root-owner-group "$deb" "$out/OOTRR-$version-linux-amd64.deb"
echo "wrote $out/OOTRR-$version-linux-amd64.deb"
