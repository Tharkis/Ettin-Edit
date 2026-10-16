#!/usr/bin/env bash
# Builds a signed APT repository from .deb files, for GitHub Pages.
# Usage: packaging/debian/build-apt-repo.sh <folder of .deb files> <output folder>
#
# APT_SIGNING_KEY must hold the armoured private key whose public half the
# .deb installs (see build-deb.sh). The output has the usual layout:
#   pool/main/*.deb
#   dists/stable/{Release,InRelease,Release.gpg}
#   dists/stable/main/binary-amd64/Packages{,.gz}
#   ettin-edit-archive-keyring.gpg (and .asc), for adding the source by hand
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
