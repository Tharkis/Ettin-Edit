#!/usr/bin/env bash
# Builds ettin-edit-<version>-1.x86_64.rpm in the current folder, for Fedora,
# openSUSE and other RPM-based systems.
# Usage: packaging/rpm/build-rpm.sh <version> <path to the ettin-edit binary>
#
# When APT_SIGNING_KEY holds the repository's armoured private key (the same
# one that signs the APT repository), the package is signed with it and adds
# the Ettin Edit RPM repository, so dnf, zypper and the software updater keep
# Ettin Edit up to date (see build-rpm-repo.sh). Needs rpmbuild (and gpg).
set -euo pipefail

version=$1
binary=$(realpath "$2")
src=$(cd "$(dirname "$0")/../.." && pwd)
top=$(mktemp -d)
trap 'rm -rf "$top"' EXIT
mkdir -p "$top/SPECS"

key="$top/RPM-GPG-KEY-ettin-edit"
repo_files=""
if [ -n "${APT_SIGNING_KEY:-}" ]; then
  export GNUPGHOME="$top/gnupg"
  mkdir -m 700 "$GNUPGHOME"
  printf '%s\n' "$APT_SIGNING_KEY" | gpg --batch --quiet --import
  fingerprint=$(gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" { print $10; exit }')
  gpg --batch --armor --export > "$key"
  repo_files='%config(noreplace) %{_sysconfdir}/yum.repos.d/ettin-edit.repo
%config(noreplace) %{_sysconfdir}/zypp/repos.d/ettin-edit.repo
%{_sysconfdir}/pki/rpm-gpg/RPM-GPG-KEY-ettin-edit'
else
  echo "::warning::APT_SIGNING_KEY is not set, so the .rpm isn't signed and won't add the Ettin Edit RPM repository."
fi

# Symbol versions marked weak (flags 0x02) in the program's version
# references, which rpm would otherwise require.
weak=$(objdump -p "$binary" | awk '
  /required from/ { lib = $3; sub(/:$/, "", lib); next }
  lib != "" && $2 == "0x02" {
    l = lib; v = $4; gsub(/\./, "\\.", l); gsub(/\./, "\\.", v)
    print "^" l "\\(" v "\\)"
  }' | paste -sd'|')
weak=${weak:-^$}
# Backslashes are halved when rpm expands the macro.
weak=${weak//\\/\\\\}

# rpm versions can't contain '-'.
rpm_version=${version//-/\~}
while IFS= read -r line; do
  case "$line" in
    @REPO_FILES@) printf '%s\n' "$repo_files" ;;
    *) line=${line//@VERSION@/"$rpm_version"}
       line=${line//@SRC@/"$src"}
       line=${line//@BINARY@/"$binary"}
       line=${line//@KEY@/"$key"}
       line=${line//@WEAK_REQUIRES@/"$weak"}
       printf '%s\n' "$line" ;;
  esac
done < "$src/packaging/rpm/ettin-edit.spec" > "$top/SPECS/ettin-edit.spec"

rpmbuild -bb --quiet \
  --define "_topdir $top" \
  --define "_build_id_links none" \
  --define "_binary_payload w9.xzdio" \
  "$top/SPECS/ettin-edit.spec"
rpm=$(find "$top/RPMS" -name 'ettin-edit-*.rpm' | head -1)

if [ -n "${APT_SIGNING_KEY:-}" ]; then
  rpmsign --addsign --define "_gpg_name $fingerprint" "$rpm" >/dev/null
fi
cp "$rpm" .
echo "Built $(basename "$rpm")"
