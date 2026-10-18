# RPM spec for Ettin Edit, filled in by build-rpm.sh, which packages a
# program that is already built (the release workflow's Linux build).
# The @NAMES@ are replaced there.
Name:           ettin-edit
Version:        @VERSION@
Release:        1
Summary:        Modern, cross-platform text editor inspired by Notepad++
License:        MIT
URL:            https://github.com/Tharkis/Ettin-Edit
BugURL:         https://github.com/Tharkis/Ettin-Edit/issues
Vendor:         Tharkis
Packager:       Tharkis <noreply@github.com>
ExclusiveArch:  x86_64
# By shared library, so the same package suits Fedora and openSUSE, whose
# package names differ. The program loads these at run time, so rpm can't
# find them by itself.
Requires:       libxkbcommon.so.0()(64bit)
Requires:       libxkbcommon-x11.so.0()(64bit)
Requires:       libGL.so.1()(64bit)
Requires:       libEGL.so.1()(64bit)
Requires:       hicolor-icon-theme
Recommends:     xdg-desktop-portal
Recommends:     xdg-utils

%global debug_package %{nil}
# glibc versions the program only uses when they're there (weak, such as
# 2.39 for pidfd_spawnp) aren't requirements; build-rpm.sh lists them.
%global __requires_exclude @WEAK_REQUIRES@
%global __strip /bin/true

%description
Ettin Edit is a fast text and code editor with tabs, split view, syntax
highlighting, multiple carets, compare, CsvQuery (SQL over CSV), JSON and
XML tools, and plugins written in Python or JavaScript.

The package adds the Ettin Edit package repository (for dnf, and for zypper
on openSUSE), so your software updater installs new versions.

%install
src=@SRC@
install -Dm755 @BINARY@ %{buildroot}%{_bindir}/ettin-edit
install -Dm644 $src/packaging/linux/ettin-edit.desktop %{buildroot}%{_datadir}/applications/ettin-edit.desktop
install -Dm644 $src/packaging/linux/io.github.tharkis.EttinEdit.metainfo.xml \
  %{buildroot}%{_datadir}/metainfo/io.github.tharkis.EttinEdit.metainfo.xml
install -Dm644 $src/assets/icons/ettin-edit.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/ettin-edit.svg
for size in 32 48 64 128 256; do
  install -Dm644 $src/assets/icons/ettin-edit-$size.png \
    %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/ettin-edit.png
done
install -Dm644 $src/LICENSE %{buildroot}%{_datadir}/licenses/ettin-edit/LICENSE
install -Dm644 $src/README.md %{buildroot}%{_docdir}/ettin-edit/README.md
if [ -f @KEY@ ]; then
  install -Dm644 @KEY@ %{buildroot}%{_sysconfdir}/pki/rpm-gpg/RPM-GPG-KEY-ettin-edit
  install -Dm644 $src/packaging/rpm/ettin-edit.repo %{buildroot}%{_sysconfdir}/yum.repos.d/ettin-edit.repo
  install -Dm644 $src/packaging/rpm/ettin-edit.repo %{buildroot}%{_sysconfdir}/zypp/repos.d/ettin-edit.repo
fi

%files
%license %{_datadir}/licenses/ettin-edit/LICENSE
%doc %{_docdir}/ettin-edit/README.md
%{_bindir}/ettin-edit
%{_datadir}/applications/ettin-edit.desktop
%{_datadir}/metainfo/io.github.tharkis.EttinEdit.metainfo.xml
%{_datadir}/icons/hicolor/*/apps/ettin-edit.*
@REPO_FILES@
