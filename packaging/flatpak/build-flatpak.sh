#!/usr/bin/env bash
# Builds ettin-edit-<version>-x86_64.flatpak in the current folder: a Flatpak
# bundle of the program that is already built (the release workflow's Linux
# build), on the freedesktop.org runtime.
# Usage: packaging/flatpak/build-flatpak.sh <version> <path to the ettin-edit binary>
#
# When APT_SIGNING_KEY holds the repository's armoured private key (the same
# one that signs the APT repository), the bundle carries its public half and
# the address of the Ettin Edit Flatpak repository, so installing it also adds
# that repository and `flatpak update` or the software centre keep Ettin Edit
# up to date (see build-flatpak-repo.sh). The runtime comes from Flathub.
# Needs flatpak, ostree and appstreamcli-compose with an SVG loader
# (appstream-compose and librsvg2-common on Ubuntu). The runtime itself isn't
# needed to build.
set -euo pipefail

version=$1
binary=$(realpath "$2")
src=$(cd "$(dirname "$0")/../.." && pwd)
app=io.github.tharkis.EttinEdit
runtime_version=24.08
out="$PWD/ettin-edit-$version-x86_64.flatpak"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
build="$work/build"
files="$build/files"

# What `flatpak build-init` would write; written by hand so the runtime
# doesn't have to be installed.
mkdir -p "$files" "$build/var"
sed -e "s/@APP@/$app/" -e "s/@RUNTIME_VERSION@/$runtime_version/" \
  "$src/packaging/flatpak/metadata" > "$build/metadata"

install -Dm755 "$binary" "$files/bin/ettin-edit"
# Exported files have to be named after the app id.
sed "s/^Icon=.*/Icon=$app/" "$src/packaging/linux/ettin-edit.desktop" \
  > "$work/$app.desktop"
install -Dm644 "$work/$app.desktop" "$files/share/applications/$app.desktop"
sed "s|<launchable type=\"desktop-id\">ettin-edit.desktop</launchable>|<launchable type=\"desktop-id\">$app.desktop</launchable>|" \
  "$src/packaging/linux/io.github.tharkis.EttinEdit.metainfo.xml" > "$work/$app.metainfo.xml"
grep -q "<launchable type=\"desktop-id\">$app.desktop</launchable>" "$work/$app.metainfo.xml"
install -Dm644 "$work/$app.metainfo.xml" "$files/share/metainfo/$app.metainfo.xml"
install -Dm644 "$src/assets/icons/ettin-edit.svg" "$files/share/icons/hicolor/scalable/apps/$app.svg"
for size in 32 48 64 128 256; do
  install -Dm644 "$src/assets/icons/ettin-edit-$size.png" \
    "$files/share/icons/hicolor/${size}x${size}/apps/$app.png"
done
install -Dm644 "$src/LICENSE" "$files/share/licenses/$app/LICENSE"
install -Dm644 "$src/README.md" "$files/share/doc/ettin-edit/README.md"

# The AppStream catalogue software centres read from the repository and the
# bundle, as flatpak-builder makes it.
appstreamcli compose --no-net --prefix=/ --origin="$app" \
  --result-root="$files" \
  --data-dir="$files/share/app-info/xmls" \
  --icons-dir="$files/share/app-info/icons/flatpak" \
  "$files"
test -f "$files/share/app-info/xmls/$app.xml.gz"

flatpak build-finish "$build" >/dev/null

repo="$work/repo"
bundle_args=(--runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo)
if [ -n "${APT_SIGNING_KEY:-}" ]; then
  export GNUPGHOME="$work/gnupg"
  mkdir -m 700 "$GNUPGHOME"
  printf '%s\n' "$APT_SIGNING_KEY" | gpg --batch --quiet --import
  fingerprint=$(gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" { print $10; exit }')
  gpg --batch --export > "$work/key.gpg"
  flatpak build-export --gpg-sign="$fingerprint" --gpg-homedir="$GNUPGHOME" \
    --subject="Ettin Edit $version" "$repo" "$build" stable
  bundle_args+=(--repo-url=https://tharkis.github.io/Ettin-Edit/flatpak/repo --gpg-keys="$work/key.gpg")
else
  echo "::warning::APT_SIGNING_KEY is not set, so the .flatpak won't add the Ettin Edit Flatpak repository and won't get updates."
  flatpak build-export --subject="Ettin Edit $version" "$repo" "$build" stable
fi
flatpak build-bundle "${bundle_args[@]}" "$repo" "$out" "$app" stable
echo "Built $(basename "$out")"
