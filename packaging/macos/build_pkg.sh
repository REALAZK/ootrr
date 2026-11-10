#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
    echo "usage: build_pkg.sh <version> <universal_binary> <out_dir>" >&2
    exit 2
fi

version="$1"
bin="$2"
out="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

root="$work/root"
app="$root/Applications/OOTRR.app/Contents"
mkdir -p "$root/usr/local/bin" "$app/MacOS" "$work/res" "$out"

install -m 755 "$bin" "$root/usr/local/bin/ootrr"

cat > "$app/MacOS/OOTRR" <<'LAUNCH'
#!/bin/sh
exec /usr/bin/osascript \
    -e 'tell application "Terminal" to activate' \
    -e 'tell application "Terminal" to do script "/usr/local/bin/ootrr serve"'
LAUNCH
chmod 755 "$app/MacOS/OOTRR"

cat > "$app/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>OOTRR</string>
    <key>CFBundleDisplayName</key>
    <string>OOTRR</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.realazk.ootrr.app</string>
    <key>CFBundleExecutable</key>
    <string>OOTRR</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
</dict>
</plist>
PLIST

pkgbuild \
    --root "$root" \
    --identifier io.github.realazk.ootrr \
    --version "$version" \
    --install-location / \
    "$work/ootrr-component.pkg"

cp "$here"/resources/*.html "$work/res/"
cp "$repo/LICENSE" "$work/res/license.txt"
sed "s/@VERSION@/$version/g" "$here/distribution.xml" > "$work/distribution.xml"

productbuild \
    --distribution "$work/distribution.xml" \
    --resources "$work/res" \
    --package-path "$work" \
    "$out/OOTRR-$version-macos-universal.pkg"

echo "wrote $out/OOTRR-$version-macos-universal.pkg"
