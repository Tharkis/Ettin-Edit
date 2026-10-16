#!/usr/bin/env bash
# Builds ettin-edit_<version>_amd64.deb in the current folder.
# Usage: packaging/debian/build-deb.sh <version> <path to the ettin-edit binary>
#
# When APT_SIGNING_KEY holds the repository's armoured private key, the package
# also installs the Ettin Edit APT source and its public key, so the system's
# software updater keeps Ettin Edit up to date (see build-apt-repo.sh). The
# public key is derived from the private one, so there is only one secret.
set -euo pipefail

version=$1
binary=$2
src=$(cd "$(dirname "$0")/../.." && pwd)
root="ettin-edit_${version}_amd64"

rm -rf "$root"
install -Dm755 "$binary" "$root/usr/bin/ettin-edit"
install -Dm644 "$src/packaging/linux/ettin-edit.desktop" "$root/usr/share/applications/ettin-edit.desktop"
install -Dm644 "$src/packaging/linux/io.github.tharkis.EttinEdit.metainfo.xml" \
  "$root/usr/share/metainfo/io.github.tharkis.EttinEdit.metainfo.xml"
install -Dm644 "$src/assets/icons/ettin-edit.svg" "$root/usr/share/icons/hicolor/scalable/apps/ettin-edit.svg"
for size in 32 48 64 128 256; do
  install -Dm644 "$src/assets/icons/ettin-edit-$size.png" \
    "$root/usr/share/icons/hicolor/${size}x${size}/apps/ettin-edit.png"
done
install -Dm644 "$src/README.md" "$root/usr/share/doc/ettin-edit/README.md"
install -Dm644 "$src/packaging/debian/copyright" "$root/usr/share/doc/ettin-edit/copyright"

mkdir -p "$root/DEBIAN"
if [ -n "${APT_SIGNING_KEY:-}" ]; then
  gnupg=$(mktemp -d)
  trap 'rm -rf "$gnupg"' EXIT
  printf '%s\n' "$APT_SIGNING_KEY" | gpg --homedir "$gnupg" --batch --quiet --import
  mkdir -p "$root/usr/share/keyrings"
  gpg --homedir "$gnupg" --batch --export > "$root/usr/share/keyrings/ettin-edit-archive-keyring.gpg"
  chmod 644 "$root/usr/share/keyrings/ettin-edit-archive-keyring.gpg"
  install -Dm644 "$src/packaging/debian/ettin-edit.sources" "$root/etc/apt/sources.list.d/ettin-edit.sources"
  echo /etc/apt/sources.list.d/ettin-edit.sources > "$root/DEBIAN/conffiles"
else
  echo "::warning::APT_SIGNING_KEY is not set, so the .deb won't add the Ettin Edit APT repository and software updaters won't update it."
fi

size=$(du -sk "$root/usr" | cut -f1)
sed -e "s/@VERSION@/$version/" -e "s/@SIZE@/$size/" "$src/packaging/debian/control" > "$root/DEBIAN/control"
dpkg-deb --root-owner-group --build "$root"
rm -rf "$root"
