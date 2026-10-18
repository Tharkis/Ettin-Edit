#!/usr/bin/env bash
# Checks that every Linux package in a folder carries what software centres,
# package managers and AppImage tools show about Ettin Edit: the AppStream
# file with the MIT licence, website and this version's release notes, the
# desktop file, the icons and the licence text. Run by the release workflow
# after the builds, so none of it can go missing without the build failing.
# Usage: packaging/check-packages.sh <folder with the builds> <version>
#
# With EXPECT_UPDATES=1 it also checks that the .deb, .rpm and .flatpak add
# the Ettin Edit repositories (which needs the signing key at build time).
# Needs appstreamcli, desktop-file-validate, dpkg-deb, rpm, rpm2cpio, cpio,
# zstd, flatpak and ostree.
set -euo pipefail

dist=$(cd "$1" && pwd)
version=$2
app=io.github.tharkis.EttinEdit
failures=0
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() {
  echo "::error::$*"
  failures=$((failures + 1))
}

# The file matching a pattern, or a failure.
found=""
find_build() {
  local matches=("$dist"/$1)
  found=${matches[0]}
  if [ ! -e "$found" ]; then
    fail "There's no $1"
    return 1
  fi
}

need_file() {
  local root=$1 path=$2 what=$3
  if [ ! -e "$root/$path" ]; then
    fail "$what has no $path"
    return 1
  fi
}

# The AppStream file: valid, with the licence, links and this version.
check_metainfo() {
  local file=$1 what=$2 desktop_id=$3
  appstreamcli validate --no-net --explain "$file" >"$work/validate.log" 2>&1 ||
    { cat "$work/validate.log"; fail "$what: the AppStream file doesn't validate"; }
  grep -q '<project_license>MIT</project_license>' "$file" || fail "$what: the AppStream file has no MIT licence"
  grep -q '<url type="homepage">https://github.com/Tharkis/Ettin-Edit</url>' "$file" || fail "$what: the AppStream file has no website"
  grep -q '<content_rating type="oars-' "$file" || fail "$what: the AppStream file has no age rating"
  grep -q "<release version=\"$version\"" "$file" || fail "$what: the AppStream file has no release notes for $version"
  grep -q "<launchable type=\"desktop-id\">$desktop_id</launchable>" "$file" ||
    fail "$what: the AppStream file doesn't point at $desktop_id"
}

check_desktop() {
  local file=$1 what=$2 icon=$3
  desktop-file-validate "$file" || fail "$what: the desktop file doesn't validate"
  grep -qx "Icon=$icon" "$file" || fail "$what: the desktop file's icon isn't $icon"
}

# A system package unpacked into $1, installed under /usr.
check_usr_tree() {
  local root=$1 what=$2
  need_file "$root" usr/bin/ettin-edit "$what"
  need_file "$root" "usr/share/metainfo/$app.metainfo.xml" "$what" &&
    check_metainfo "$root/usr/share/metainfo/$app.metainfo.xml" "$what" ettin-edit.desktop
  need_file "$root" usr/share/applications/ettin-edit.desktop "$what" &&
    check_desktop "$root/usr/share/applications/ettin-edit.desktop" "$what" ettin-edit
  need_file "$root" usr/share/icons/hicolor/scalable/apps/ettin-edit.svg "$what"
  need_file "$root" usr/share/icons/hicolor/256x256/apps/ettin-edit.png "$what"
}

echo "== Debian package"
if find_build "ettin-edit_${version}_amd64.deb"; then
  deb=$found
  dpkg-deb -x "$deb" "$work/deb"
  check_usr_tree "$work/deb" .deb
  for field in Homepage Description Maintainer; do
    [ -n "$(dpkg-deb -f "$deb" "$field")" ] || fail ".deb: no $field field"
  done
  grep -q '^License: Expat' "$work/deb/usr/share/doc/ettin-edit/copyright" || fail ".deb: the copyright file has no MIT (Expat) licence"
  if [ "${EXPECT_UPDATES:-}" = 1 ]; then
    need_file "$work/deb" etc/apt/sources.list.d/ettin-edit.sources .deb
    need_file "$work/deb" usr/share/keyrings/ettin-edit-archive-keyring.gpg .deb
  fi
fi

echo "== RPM package"
if find_build "ettin-edit-$version-*.x86_64.rpm"; then
  rpm=$found
  mkdir "$work/rpm"
  (cd "$work/rpm" && rpm2cpio "$rpm" | cpio -idm --quiet)
  check_usr_tree "$work/rpm" .rpm
  need_file "$work/rpm" usr/share/licenses/ettin-edit/LICENSE .rpm
  [ "$(rpm -qp --qf '%{LICENSE}' "$rpm" 2>/dev/null)" = MIT ] || fail ".rpm: the License tag isn't MIT"
  [ "$(rpm -qp --qf '%{URL}' "$rpm" 2>/dev/null)" = https://github.com/Tharkis/Ettin-Edit ] || fail ".rpm: the URL tag isn't the website"
  [ "$(rpm -qp --qf '%{VERSION}' "$rpm" 2>/dev/null)" = "$version" ] || fail ".rpm: the version isn't $version"
  if [ "${EXPECT_UPDATES:-}" = 1 ]; then
    need_file "$work/rpm" etc/yum.repos.d/ettin-edit.repo .rpm
    need_file "$work/rpm" etc/zypp/repos.d/ettin-edit.repo .rpm
    need_file "$work/rpm" etc/pki/rpm-gpg/RPM-GPG-KEY-ettin-edit .rpm
    rpm -qip "$rpm" 2>/dev/null > "$work/rpm-info"
    grep -q '^Signature *: .*Key ID' "$work/rpm-info" || fail ".rpm: the package isn't signed"
  fi
fi

echo "== Arch package"
if find_build "ettin-edit-$version-*-x86_64.pkg.tar.zst"; then
  pkg=$found
  mkdir "$work/arch"
  tar -I zstd -xf "$pkg" -C "$work/arch"
  check_usr_tree "$work/arch" "Arch package"
  need_file "$work/arch" usr/share/licenses/ettin-edit/LICENSE "Arch package"
  grep -qx 'license = MIT' "$work/arch/.PKGINFO" || fail "Arch package: the licence isn't MIT"
  grep -qx 'url = https://github.com/Tharkis/Ettin-Edit' "$work/arch/.PKGINFO" || fail "Arch package: no website"
fi

echo "== Linux .tar.gz"
if find_build "ettin-edit-$version-linux-x86_64.tar.gz"; then
  tgz=$found
  tar -xzf "$tgz" -C "$work"
  t="$work/ettin-edit-$version-linux-x86_64"
  for f in ettin-edit LICENSE README.md ettin-edit.desktop "$app.metainfo.xml" icons/ettin-edit.svg icons/ettin-edit-256.png; do
    need_file "$t" "$f" .tar.gz
  done
  [ -f "$t/$app.metainfo.xml" ] && check_metainfo "$t/$app.metainfo.xml" .tar.gz ettin-edit.desktop
fi

echo "== AppImage"
if find_build "ettin-edit-$version-x86_64.AppImage"; then
  appimage=$found
  [ -f "$appimage.zsync" ] || fail "AppImage: no .zsync file for updates"
  chmod +x "$appimage"
  update=$("$appimage" --appimage-updateinformation 2>/dev/null || true)
  [ "$update" = "gh-releases-zsync|Tharkis|Ettin-Edit|latest|ettin-edit-*-x86_64.AppImage.zsync" ] ||
    fail "AppImage: wrong update information '$update'"
  (cd "$work" && "$appimage" --appimage-extract >/dev/null)
  a="$work/squashfs-root"
  need_file "$a" AppRun AppImage
  need_file "$a" usr/bin/ettin-edit AppImage
  need_file "$a" usr/share/licenses/ettin-edit/LICENSE AppImage
  need_file "$a" ettin-edit.png AppImage
  need_file "$a" "$app.desktop" AppImage && check_desktop "$a/$app.desktop" AppImage ettin-edit
  need_file "$a" "usr/share/metainfo/$app.appdata.xml" AppImage &&
    check_metainfo "$a/usr/share/metainfo/$app.appdata.xml" AppImage "$app.desktop"
fi

echo "== Flatpak bundle"
if find_build "ettin-edit-$version-x86_64.flatpak"; then
  flatpak=$found
  ostree init --repo="$work/ostree" --mode=bare-user-only
  flatpak build-import-bundle --no-update-summary "$work/ostree" "$flatpak" >/dev/null 2>&1 ||
    fail "Flatpak: the bundle can't be imported"
  ref="app/$app/x86_64/stable"
  if ostree --repo="$work/ostree" checkout -U "$ref" "$work/fp" 2>/dev/null; then
    f="$work/fp"
    grep -qx "command=ettin-edit" "$f/metadata" || fail "Flatpak: the metadata doesn't run ettin-edit"
    grep -q "^runtime=org.freedesktop.Platform/" "$f/metadata" || fail "Flatpak: not on the freedesktop runtime"
    need_file "$f" files/bin/ettin-edit Flatpak
    need_file "$f" "files/share/licenses/$app/LICENSE" Flatpak
    need_file "$f" "export/share/icons/hicolor/scalable/apps/$app.svg" Flatpak
    need_file "$f" "export/share/icons/hicolor/256x256/apps/$app.png" Flatpak
    need_file "$f" "export/share/applications/$app.desktop" Flatpak &&
      check_desktop "$f/export/share/applications/$app.desktop" Flatpak "$app"
    need_file "$f" "export/share/metainfo/$app.metainfo.xml" Flatpak &&
      check_metainfo "$f/export/share/metainfo/$app.metainfo.xml" Flatpak "$app.desktop"
    need_file "$f" "files/share/app-info/xmls/$app.xml.gz" Flatpak &&
      { zgrep -q '<project_license>MIT</project_license>' "$f/files/share/app-info/xmls/$app.xml.gz" ||
        fail "Flatpak: the AppStream catalogue has no MIT licence"; }
  else
    fail "Flatpak: the bundle has no $ref"
  fi
  if [ "${EXPECT_UPDATES:-}" = 1 ]; then
    # The address is in the bundle's header.
    head -c 1000000 "$flatpak" > "$work/flatpak-header"
    grep -aq 'https://tharkis.github.io/Ettin-Edit/flatpak/repo' "$work/flatpak-header" ||
      fail "Flatpak: the bundle doesn't add the Ettin Edit Flatpak repository"
  fi
fi

echo "== Windows .zip"
if find_build "ettin-edit-$version-windows-x86_64.zip"; then
  zip=$found
  for f in ettin-edit.exe LICENSE README.md; do
    unzip -Z1 "$zip" > "$work/zip-list"
    grep -q "/$f\$" "$work/zip-list" || fail "Windows .zip has no $f"
  done
fi

if [ "$failures" -gt 0 ]; then
  echo "$failures problem(s) with the packages."
  exit 1
fi
echo "All packages carry their metadata."
