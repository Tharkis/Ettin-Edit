#!/usr/bin/env bash
# Builds a signed Flatpak repository from a release's .flatpak bundle, for
# GitHub Pages. Installing the bundle adds this repository (see
# build-flatpak.sh), so `flatpak update` and software centres find new versions.
# Usage: packaging/flatpak/build-flatpak-repo.sh <.flatpak file> <output folder>
#
# APT_SIGNING_KEY must hold the armoured private key (the one that signs the
# APT repository). The output has:
#   repo/                     the OSTree repository, with its AppStream catalogue
#   ettin-edit.flatpakref     opens in a software centre to install Ettin Edit
#   ettin-edit.flatpakrepo    adds the repository
# Needs flatpak, ostree and gpg.
set -euo pipefail

bundle=$(realpath "$1")
mkdir -p "$2"
out=$(cd "$2" && pwd)
app=io.github.tharkis.EttinEdit
url=https://tharkis.github.io/Ettin-Edit/flatpak
: "${APT_SIGNING_KEY:?APT_SIGNING_KEY must hold the repository signing key}"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export GNUPGHOME="$work/gnupg"
mkdir -m 700 "$GNUPGHOME"
printf '%s\n' "$APT_SIGNING_KEY" | gpg --batch --quiet --import
fingerprint=$(gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" { print $10; exit }')
key=$(gpg --batch --export | base64 -w0)
sign=(--gpg-sign="$fingerprint" --gpg-homedir="$GNUPGHOME")

# The bundle's commit, so a repository rebuilt from the same release offers
# the same version and nobody is told about an update that isn't one.
rm -rf "$out/repo"
ostree init --repo="$out/repo" --mode=archive-z2
flatpak build-import-bundle --no-update-summary "${sign[@]}" "$out/repo" "$bundle"
flatpak build-update-repo --title="Ettin Edit" --generate-static-deltas --prune \
  --default-branch=stable "${sign[@]}" "$out/repo"
ostree --repo="$out/repo" refs | grep -qx "app/$app/x86_64/stable"

cat > "$out/ettin-edit.flatpakref" <<REF
[Flatpak Ref]
Name=$app
Branch=stable
Title=Ettin Edit
Url=$url/repo
SuggestRemoteName=ettin-edit
Homepage=https://github.com/Tharkis/Ettin-Edit
Icon=https://raw.githubusercontent.com/Tharkis/Ettin-Edit/main/assets/icons/ettin-edit-128.png
RuntimeRepo=https://dl.flathub.org/repo/flathub.flatpakrepo
IsRuntime=false
GPGKey=$key
REF

cat > "$out/ettin-edit.flatpakrepo" <<REPO
[Flatpak Repo]
Title=Ettin Edit
Url=$url/repo
Homepage=https://github.com/Tharkis/Ettin-Edit
Comment=The Ettin Edit text editor
Icon=https://raw.githubusercontent.com/Tharkis/Ettin-Edit/main/assets/icons/ettin-edit-128.png
GPGKey=$key
REPO
