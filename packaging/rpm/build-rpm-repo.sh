#!/usr/bin/env bash
# Builds a signed RPM repository (for dnf and zypper) from .rpm files, for
# GitHub Pages.
# Usage: packaging/rpm/build-rpm-repo.sh <folder of .rpm files> <output folder>
#
# APT_SIGNING_KEY must hold the armoured private key that signed the packages
# (see build-rpm.sh). The output has:
#   *.rpm
#   repodata/ (repomd.xml, signed as repomd.xml.asc, and the AppStream
#     catalogue that software centres read)
#   RPM-GPG-KEY-ettin-edit and ettin-edit.repo, for adding the repository by hand
# Needs createrepo_c, gpg and, for the AppStream catalogue, appstreamcli-compose
# with an SVG loader (appstream-compose and librsvg2-common on Ubuntu) and
# rpm2cpio.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
rpms=$(cd "$1" && pwd)
mkdir -p "$2"
out=$(cd "$2" && pwd)
: "${APT_SIGNING_KEY:?APT_SIGNING_KEY must hold the repository signing key}"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export GNUPGHOME="$work/gnupg"
mkdir -m 700 "$GNUPGHOME"
printf '%s\n' "$APT_SIGNING_KEY" | gpg --batch --quiet --import
fingerprint=$(gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" { print $10; exit }')

cp "$rpms"/*.rpm "$out/"
createrepo_c --quiet --update "$out"

# The AppStream catalogue, from the newest package's metainfo and icons, so
# GNOME Software and KDE Discover show Ettin Edit's licence, screenshots and
# release notes. dnf's modifyrepo_c adds it to repomd.xml.
newest=$(ls "$out"/ettin-edit-*.rpm | sort -V | tail -1)
mkdir "$work/tree"
(cd "$work/tree" && rpm2cpio "$newest" | cpio -idm --quiet)
appstreamcli compose --no-net --origin=ettin-edit --result-root="$work/compose" --prefix=/usr "$work/tree"
zcat "$work/compose/usr/share/swcatalog/xml/ettin-edit.xml.gz" |
  sed -e 's|^<components version="\([^"]*\)">|<components version="\1" origin="ettin-edit">|' \
      -e 's|^\( *\)<id>io.github.tharkis.EttinEdit</id>|&\n\1<pkgname>ettin-edit</pkgname>|' > "$work/appstream.xml"
grep -q '<pkgname>ettin-edit</pkgname>' "$work/appstream.xml"
tar -cf "$work/appstream-icons.tar" -C "$work/compose/usr/share/swcatalog/icons/ettin-edit" .
modifyrepo_c --compress-type=gz --mdtype=appstream "$work/appstream.xml" "$out/repodata"
modifyrepo_c --compress-type=gz --mdtype=appstream-icons "$work/appstream-icons.tar" "$out/repodata"

gpg --batch --yes --local-user "$fingerprint" --armor --detach-sign \
  -o "$out/repodata/repomd.xml.asc" "$out/repodata/repomd.xml"
gpg --batch --armor --export > "$out/RPM-GPG-KEY-ettin-edit"
# For adding the repository by hand: the same file, with the key from here.
sed 's|^gpgkey=.*|gpgkey=https://tharkis.github.io/Ettin-Edit/rpm/RPM-GPG-KEY-ettin-edit|' \
  "$here/ettin-edit.repo" > "$out/ettin-edit.repo"
