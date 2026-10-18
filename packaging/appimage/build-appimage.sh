#!/usr/bin/env bash
# Builds ettin-edit-<version>-x86_64.AppImage (and its .zsync) in the current
# folder from the program that is already built (the release workflow's Linux
# build).
# Usage: packaging/appimage/build-appimage.sh <version> <path to the ettin-edit binary>
#
# Needs appimagetool (on the PATH, or set APPIMAGETOOL to it) and zsyncmake.
# Like the .tar.gz, the AppImage uses the system's OpenGL and libxkbcommon,
# which every desktop has. It carries update information, so AppImage
# updaters (Gear Lever, AppImageUpdate) and Ettin Edit itself can update it.
set -euo pipefail

version=$1
binary=$(realpath "$2")
src=$(cd "$(dirname "$0")/../.." && pwd)
app=io.github.tharkis.EttinEdit
tool=${APPIMAGETOOL:-appimagetool}
out="$PWD/ettin-edit-$version-x86_64.AppImage"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
dir="$work/Ettin_Edit.AppDir"

install -Dm755 "$binary" "$dir/usr/bin/ettin-edit"
ln -s usr/bin/ettin-edit "$dir/AppRun"
# appimagetool wants the desktop file and its icon at the top, and the
# AppStream file named after the desktop file.
install -Dm644 "$src/packaging/linux/ettin-edit.desktop" "$dir/usr/share/applications/$app.desktop"
ln -s "usr/share/applications/$app.desktop" "$dir/$app.desktop"
sed "s|<launchable type=\"desktop-id\">ettin-edit.desktop</launchable>|<launchable type=\"desktop-id\">$app.desktop</launchable>|" \
  "$src/packaging/linux/io.github.tharkis.EttinEdit.metainfo.xml" > "$work/metainfo.xml"
grep -q "<launchable type=\"desktop-id\">$app.desktop</launchable>" "$work/metainfo.xml"
install -Dm644 "$work/metainfo.xml" "$dir/usr/share/metainfo/$app.appdata.xml"
install -Dm644 "$src/assets/icons/ettin-edit.svg" "$dir/usr/share/icons/hicolor/scalable/apps/ettin-edit.svg"
for size in 32 48 64 128 256; do
  install -Dm644 "$src/assets/icons/ettin-edit-$size.png" \
    "$dir/usr/share/icons/hicolor/${size}x${size}/apps/ettin-edit.png"
done
ln -s usr/share/icons/hicolor/256x256/apps/ettin-edit.png "$dir/ettin-edit.png"
ln -s ettin-edit.png "$dir/.DirIcon"
install -Dm644 "$src/LICENSE" "$dir/usr/share/licenses/ettin-edit/LICENSE"
install -Dm644 "$src/README.md" "$dir/usr/share/doc/ettin-edit/README.md"

# Where AppImage updaters look for the newest version. The AppStream file is
# checked separately (without the network) by packaging/check-packages.sh.
update="gh-releases-zsync|Tharkis|Ettin-Edit|latest|ettin-edit-*-x86_64.AppImage.zsync"
(cd "$work" && ARCH=x86_64 VERSION="$version" "$tool" --no-appstream -u "$update" "$dir" "$out")
# appimagetool writes the .zsync next to where it was run.
mv "$work/$(basename "$out").zsync" "$out.zsync"
echo "Built $(basename "$out")"
