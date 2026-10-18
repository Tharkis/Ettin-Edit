#!/usr/bin/env bash
# Builds a signed APT repository from .deb files, for GitHub Pages.
# Usage: packaging/debian/build-apt-repo.sh <folder of .deb files> <output folder>
#
# APT_SIGNING_KEY must hold the armoured private key whose public half the
# .deb installs (see build-deb.sh). The output has the usual layout:
#   pool/main/*.deb
#   dists/stable/{Release,InRelease,Release.gpg}
#   dists/stable/main/binary-amd64/Packages{,.gz}
#   dists/stable/main/dep11/{Components-amd64.yml,icons-*.tar}{,.gz}
#   ettin-edit-archive-keyring.gpg (and .asc), for adding the source by hand
#
# The dep11 folder is the AppStream catalogue that software centres (GNOME
# Software, KDE Discover, Ubuntu's App Center) read to show Ettin Edit's
# licence, screenshots and release notes once this repository is added. It is
# composed from the newest .deb, so it needs appstreamcli-compose
# (appstream-compose) and an SVG loader for its icons (librsvg2-common).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
debs=$(cd "$1" && pwd)
mkdir -p "$2"
out=$(cd "$2" && pwd)
: "${APT_SIGNING_KEY:?APT_SIGNING_KEY must hold the repository signing key}"

gnupg=$(mktemp -d)
trap 'rm -rf "$gnupg"' EXIT
printf '%s\n' "$APT_SIGNING_KEY" | gpg --homedir "$gnupg" --batch --quiet --import
fingerprint=$(gpg --homedir "$gnupg" --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" { print $10; exit }')

mkdir -p "$out/pool/main" "$out/dists/stable/main/binary-amd64"
cp "$debs"/*.deb "$out/pool/main/"
cd "$out"
apt-ftparchive packages pool > dists/stable/main/binary-amd64/Packages
gzip -9kf dists/stable/main/binary-amd64/Packages

# The AppStream catalogue, from the newest package's metainfo and icons.
newest=$(ls pool/main/*.deb | sort -V | tail -1)
tree="$gnupg/tree"
dpkg-deb -x "$newest" "$tree"
appstreamcli compose --no-net --origin=ettin-edit --result-root="$gnupg/compose" --prefix=/usr "$tree"
dep11=dists/stable/main/dep11
mkdir -p "$dep11"
# The catalogue has to name its origin (the folder its icons go in) and the
# package each app comes from. Screenshot links must be relative to
# MediaBaseUrl: AppStream puts a base in front of every link when reading.
zcat "$gnupg/compose/usr/share/swcatalog/xml/ettin-edit.xml.gz" |
  sed -e 's|^<components version="\([^"]*\)">|<components version="\1" origin="ettin-edit" media_baseurl="https://raw.githubusercontent.com/Tharkis/Ettin-Edit/main/">|' \
      -e 's|^\( *\)<id>io.github.tharkis.EttinEdit</id>|&\n\1<pkgname>ettin-edit</pkgname>|' > "$gnupg/catalog.xml"
grep -q 'origin="ettin-edit"' "$gnupg/catalog.xml"
grep -q '<pkgname>ettin-edit</pkgname>' "$gnupg/catalog.xml"
appstreamcli convert --format=yaml "$gnupg/catalog.xml" "$dep11/Components-amd64.yml"
sed -i 's|^\( *url: \)https://raw.githubusercontent.com/Tharkis/Ettin-Edit/main/|\1|' "$dep11/Components-amd64.yml"
# apt only fetches an index whose uncompressed name is in Release, so keep both.
gzip -9kf "$dep11/Components-amd64.yml"
for size in 48x48 64x64 128x128 64x64@2 128x128@2; do
  icons="$gnupg/compose/usr/share/swcatalog/icons/ettin-edit/$size"
  tar -cf "$dep11/icons-$size.tar" -C "$icons" $(ls "$icons")
  gzip -9kf "$dep11/icons-$size.tar"
done
apt-ftparchive \
  -o APT::FTPArchive::Release::Origin="Ettin Edit" \
  -o APT::FTPArchive::Release::Label="Ettin Edit" \
  -o APT::FTPArchive::Release::Suite=stable \
  -o APT::FTPArchive::Release::Codename=stable \
  -o APT::FTPArchive::Release::Architectures=amd64 \
  -o APT::FTPArchive::Release::Components=main \
  -o APT::FTPArchive::Release::Description="Ettin Edit text editor" \
  release dists/stable > "$gnupg/Release"
mv "$gnupg/Release" dists/stable/Release
gpg --homedir "$gnupg" --batch --yes --local-user "$fingerprint" --clearsign \
  -o dists/stable/InRelease dists/stable/Release
gpg --homedir "$gnupg" --batch --yes --local-user "$fingerprint" --armor --detach-sign \
  -o dists/stable/Release.gpg dists/stable/Release
gpg --homedir "$gnupg" --batch --export > ettin-edit-archive-keyring.gpg
gpg --homedir "$gnupg" --batch --armor --export > ettin-edit-archive-keyring.asc
cp "$here/ettin-edit.sources" .
