#!/bin/sh
# Installs the latest devpit release, or updates an installed one.
#
#     curl -fsSL https://raw.githubusercontent.com/jholhewres/devpit/main/install.sh | sh
#
# What it does, in order, and nothing else:
#
#   1. asks GitHub which release is the latest, or takes DEVPIT_VERSION;
#   2. downloads the one file that fits this machine, and SHA256SUMS;
#   3. refuses to go on unless the file matches its checksum;
#   4. installs it — the .deb through apt on Debian and Ubuntu, the AppImage
#      into ~/.local/bin elsewhere on Linux, the .app into Applications on
#      macOS.
#
# Once installed, devpit keeps itself up to date: it checks for a new release
# when it opens and once a day after. This script is for the first install, or
# for catching up by hand.
#
# Settings, all optional:
#
#   DEVPIT_VERSION=0.1.7    install that version instead of the latest
#   DEVPIT_FORMAT=appimage  on Linux, take the AppImage even where apt exists
#   DEVPIT_DRY_RUN=1        download and verify, then say what would happen
#
# Everything is inside `main`, called on the last line: a download cut off
# half way through is then a script that defines a function and runs nothing,
# rather than one that runs its first half.

set -eu

REPO="jholhewres/devpit"

say() { printf 'devpit: %s\n' "$*"; }
fail() {
  printf 'devpit: %s\n' "$*" >&2
  exit 1
}

fetch() {
  # HTTPS only, and never a downgrade to anything older than TLS 1.2.
  curl --proto '=https' --tlsv1.2 -fsSL "$@"
}

latest_version() {
  # The `latest` page redirects to the tag, so its final address names the
  # version — no API, no rate limit, and nothing to parse but a URL.
  where=$(curl --proto '=https' --tlsv1.2 -fsSLI -o /dev/null -w '%{url_effective}' \
    "https://github.com/$REPO/releases/latest") || return 1
  tag=${where##*/}
  case "$tag" in
    v[0-9]*) printf '%s\n' "${tag#v}" ;;
    *) return 1 ;;
  esac
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{ print $1 }'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{ print $1 }'
  else
    fail "neither sha256sum nor shasum is installed, so the download cannot be checked"
  fi
}

# Which file fits this machine, and how it goes in. Sets `asset` and `how`.
choose() {
  version=$1
  os=$(uname -s)
  arch=$(uname -m)
  case "$os" in
    Linux)
      case "$arch" in
        x86_64 | amd64) deb_arch=amd64 image_arch=amd64 ;;
        aarch64 | arm64) deb_arch=arm64 image_arch=aarch64 ;;
        *) fail "no Linux build for $arch — only x86_64 and aarch64 are released" ;;
      esac
      # The .deb where apt can take it: apt brings the WebKit libraries devpit
      # needs, and puts devpit in the application menu. An AppImage carries
      # its own libraries but needs FUSE, which recent Ubuntu does not ship.
      if [ "${DEVPIT_FORMAT:-}" != "appimage" ] &&
        command -v dpkg >/dev/null 2>&1 && command -v apt-get >/dev/null 2>&1; then
        asset="devpit_${version}_${deb_arch}.deb"
        how=deb
      else
        asset="devpit_${version}_${image_arch}.AppImage"
        how=appimage
      fi
      ;;
    Darwin)
      # One universal build for both kinds of Mac. The tarball rather than the
      # .dmg: it is the same .app, it unpacks without mounting anything, and it
      # is what devpit's own updater installs.
      asset="devpit.app.tar.gz"
      how=macos
      ;;
    *)
      fail "no build for $os — devpit is released for Linux and macOS"
      ;;
  esac
}

install_deb() {
  file=$1
  version=$2
  installed=$(dpkg-query -W -f='${Version}' devpit 2>/dev/null || true)
  if [ "$installed" = "$version" ]; then
    say "devpit $version is already installed."
    return 0
  fi
  # apt reads the package as the unprivileged `_apt` user, which cannot see
  # into a folder only this user can open; without this it warns and falls
  # back to reading it as root.
  chmod 755 "$(dirname "$file")"
  chmod 644 "$file"
  if [ "$(id -u)" -eq 0 ]; then
    apt-get install -y "$file"
  else
    say "installing through apt, which asks for your password"
    sudo apt-get install -y "$file"
  fi
  say "devpit $version is installed. Open it from your applications."
  if [ -n "$installed" ]; then
    say "if devpit was open, close it and open it again to run $version."
  fi
}

install_appimage() {
  file=$1
  version=$2
  bin="$HOME/.local/bin"
  mkdir -p "$bin"
  # Under one name that does not change, so devpit replacing itself on update
  # replaces this same file rather than leaving versions to pile up.
  mv "$file" "$bin/devpit"
  chmod 755 "$bin/devpit"
  say "devpit $version is at $bin/devpit"
  # `ldconfig` lives in /sbin, which is not on every user's PATH.
  if ! { ldconfig -p 2>/dev/null || /sbin/ldconfig -p 2>/dev/null; } | grep -q 'libfuse\.so\.2'; then
    say "an AppImage needs FUSE 2, which this machine does not seem to have."
    say "on Ubuntu and Debian: sudo apt install libfuse2"
  fi
  case ":$PATH:" in
    *":$bin:"*) say "run it with: devpit" ;;
    *) say "$bin is not on your PATH — run it with: $bin/devpit" ;;
  esac
}

install_macos() {
  file=$1
  version=$2
  dir=$(dirname "$file")
  tar -xzf "$file" -C "$dir"
  [ -d "$dir/devpit.app" ] || fail "the download did not hold devpit.app"
  if [ -w /Applications ]; then
    apps=/Applications
  else
    apps="$HOME/Applications"
    mkdir -p "$apps"
  fi
  target="$apps/devpit.app"
  installed=$(defaults read "$target/Contents/Info" CFBundleShortVersionString 2>/dev/null || true)
  if [ "$installed" = "$version" ]; then
    say "devpit $version is already installed."
    return 0
  fi
  # Only this exact path, and only once the new copy is known to be there.
  rm -rf "$target"
  mv "$dir/devpit.app" "$target"
  say "devpit $version is in $apps"
  say "the app is not notarised; if macOS refuses to open it, right-click it and choose Open."
}

main() {
  command -v curl >/dev/null 2>&1 || fail "curl is needed to download devpit"

  version=${DEVPIT_VERSION:-}
  if [ -z "$version" ]; then
    version=$(latest_version) || fail "could not find the latest release on GitHub"
  fi
  version=${version#v}

  choose "$version"

  work=$(mktemp -d)
  # The folder goes on the way out, however that is. INT and TERM exit rather
  # than clean up and carry on: a trap that only tidies would leave Ctrl-C
  # tidying and then installing anyway.
  trap 'rm -rf "$work"' EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  base="https://github.com/$REPO/releases/download/v$version"
  say "downloading $asset ($version)"
  fetch -o "$work/$asset" "$base/$asset" || fail "could not download $base/$asset"
  fetch -o "$work/SHA256SUMS" "$base/SHA256SUMS" || fail "could not download the checksums for $version"

  expected=$(awk -v name="$asset" '$2 == name { print $1; exit }' "$work/SHA256SUMS")
  [ -n "$expected" ] || fail "$asset is not in the release's SHA256SUMS, so it cannot be checked — refusing to install it"
  actual=$(sha256_of "$work/$asset")
  [ "$actual" = "$expected" ] || fail "$asset does not match its checksum — refusing to install it"
  say "checksum matches"

  if [ -n "${DEVPIT_DRY_RUN:-}" ]; then
    say "dry run: would install $asset as $how, and stop here"
    return 0
  fi

  case "$how" in
    deb) install_deb "$work/$asset" "$version" ;;
    appimage) install_appimage "$work/$asset" "$version" ;;
    macos) install_macos "$work/$asset" "$version" ;;
  esac

  say "from here devpit keeps itself up to date: it checks when it opens and once a day."
}

main "$@"
