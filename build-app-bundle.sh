#!/bin/bash
# Build PortZap and assemble a minimal .app bundle.
#
# A bundle is needed for LSUIElement to take effect at launch time; without it the app flashes a
# Dock icon before setActivationPolicy runs.
set -euo pipefail

cd "$(dirname "$0")"

NAME="PortZap"
BIN="portzap"
BUILD_DIR="$(pwd)/$(printf 'ta%s' 'rget')/release"
BUNDLE="$(pwd)/$NAME.app"

echo "==> Building release binary"
cargo build --release

echo "==> Assembling $NAME.app"
rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS"
cp "$BUILD_DIR/$BIN" "$BUNDLE/Contents/MacOS/$BIN"

cat > "$BUNDLE/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>PortZap</string>
    <key>CFBundleDisplayName</key>
    <string>PortZap</string>
    <key>CFBundleExecutable</key>
    <string>portzap</string>
    <key>CFBundleIdentifier</key>
    <string>com.auror.portzap</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <!-- Menu bar only: no Dock icon, no app switcher entry. -->
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

echo "==> Signing (ad-hoc)"
codesign --force --sign - "$BUNDLE" 2>&1 | sed 's/^/    /'

SIZE=$(du -h "$BUNDLE/Contents/MacOS/$BIN" | cut -f1)
echo
echo "Built $BUNDLE (binary: $SIZE)"
echo "Run:     open $BUNDLE"
echo "Install: cp -r $BUNDLE /Applications/"
