#!/bin/sh
# Installs the latest devpit release, or updates an installed one.
#
#     curl -fsSL https://devpit.app/install.sh | sh
#
# What it does, in order, and nothing else:
#
#   1. asks GitHub which release is the latest, or takes DEVPIT_VERSION;
#   2. downloads the one file that fits this machine, and SHA256SUMS;
#   3. refuses to go on unless the file matches its checksum, and, where
#      minisign is installed, its signature by devpit's release key;
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
# On a terminal it draws a banner, a progress bar and each step, in colour
# unless NO_COLOR is set. Piped, in CI or with TERM=dumb it prints plain
# `devpit:` lines, so a log reads the same as ever.
#
# Everything is inside `main`, called on the last line: a download cut off
# half way through is then a script that defines a function and runs nothing,
# rather than one that runs its first half.

set -eu

REPO="jholhewres/devpit"
# The key every release is signed with: the updater's own (`pubkey` in
# apps/desktop/tauri.conf.json, which a guard keeps this equal to).
PUBKEY="RWQkWKDUdIb0t1UL0cbl8d7MLNvFlyj6Sq+Fb3CzQlQ772LUDqOjQdi+"
DOCS="https://devpit.app/docs"

# How it talks. `rich` only when stdout is a terminal that can take it; every
# other case keeps the plain lines a log or a pipe has always had.
ui() {
  rich='' open='' now='' meter='' said='' utf=''
  accent='' bold='' dim='' green='' red='' yellow='' off='' wipe='' hide='' show=''
  tick='+' cross='x' skip='-' busy='.' full='#' empty='-' dot='-' arrow='->' more='...'
  [ -t 1 ] && [ -z "${CI:-}" ] && [ "${TERM:-dumb}" != dumb ] || return 0
  rich=1
  e=$(printf '\033')
  wipe="${e}[K" hide="${e}[?25l" show="${e}[?25h"
  if [ -z "${NO_COLOR:-}" ]; then
    case "${COLORTERM:-}:${TERM:-}" in
      truecolor:* | 24bit:*) accent="${e}[38;2;226;121;91m" ;;
      *256*) accent="${e}[38;5;173m" ;;
      *) accent="${e}[33m" ;;
    esac
    bold="${e}[1m" dim="${e}[2m" green="${e}[32m" red="${e}[31m" yellow="${e}[33m" off="${e}[0m"
  fi
  case "${LC_ALL:-${LC_CTYPE:-${LANG:-}}}" in
    *[Uu][Tt][Ff]-8* | *[Uu][Tt][Ff]8*) utf=1 ;;
  esac
  if [ -n "$utf" ]; then
    tick='✓' cross='✗' skip='–' busy='·' full='━' empty='─' dot='·' arrow='→' more='…'
    # In colour the rest of the bar is the same line, dimmed.
    [ -z "$off" ] || empty='━'
  fi
  # stdin is the script itself under `curl | sh`, so the size comes from the tty.
  cols=$({ stty size </dev/tty; } 2>/dev/null | awk '{ print $2 }')
  case "$cols" in "" | 0 | *[!0-9]*) cols=${COLUMNS:-80} ;; esac
  case "$cols" in "" | *[!0-9]*) cols=80 ;; esac
  [ "$cols" -le 100 ] || cols=100
}

# A note: a plain line, or a dim one under the step it belongs to.
say() {
  if [ -n "$rich" ]; then
    settle
    printf '    %s%s%s\n' "$dim" "$*" "$off"
  else
    printf 'devpit: %s\n' "$*"
  fi
}

fail() {
  if [ -n "${rich:-}" ]; then
    unopen
    if [ -n "$now" ]; then
      printf '  %s%s%s %s\n' "$red" "$cross" "$off" "$now"
      printf '    %s%s%s\n' "$red" "$*" "$off" >&2
    else
      printf '  %s%s %s%s\n' "$red" "$cross" "$*" "$off" >&2
    fi
    [ -z "$said" ] || printf '    %s\n' "$said" >&2
  else
    printf 'devpit: %s\n' "$*" >&2
  fi
  exit 1
}

# A step under way, given as its plain line (or nothing) and as shown on a
# terminal, where it stays until it ticks, crosses or skips.
step() {
  now=$2
  if [ -z "$rich" ]; then
    [ -z "$1" ] || printf 'devpit: %s\n' "$1"
    return 0
  fi
  printf '  %s%s %s%s%s' "$dim" "$busy" "$2" "$more" "$off"
  open=1
}

# A step done, the same two ways: ticked, or skipped.
passed() { mark "$green$tick" "$@"; }
skipped() { mark "$yellow$skip" "$@"; }
mark() {
  if [ -z "$rich" ]; then
    [ -z "$2" ] || printf 'devpit: %s\n' "$2"
    return 0
  fi
  unopen
  printf '  %s%s %s\n' "$1" "$off" "$3"
  now=''
}

# The open step line, wiped so its result takes its place.
unopen() {
  if [ -n "$open" ]; then
    printf '\r%s' "$wipe"
    open=''
  fi
}

# The open step line, ended, when something else has to print under it.
settle() {
  if [ -n "$open" ]; then
    printf '\n'
    open=''
  fi
}

banner() {
  [ -n "$rich" ] || return 0
  if [ -z "$utf" ] || [ "$cols" -lt 66 ]; then
    printf '\n  %s%sdevpit%s  %sdesktop installer%s\n\n' "$bold" "$accent" "$off" "$dim" "$off"
    return 0
  fi
  printf '\n  %s%s%s\n' "$accent" '    ██                          ▄▄    ██' "$off"
  printf '  %s%s%s    %sdevpit%s\n' "$accent" '▄▄▄▄██  ▄▄▄▄▄▄  ▄▄  ▄▄  ▄▄▄▄▄▄  ▄▄  ▄▄██▄▄' "$off" "$bold" "$off"
  printf '  %s%s%s    %sdesktop installer%s\n' "$accent" '██  ██  ██▄▄██  ██  ██  ██  ██  ██    ██  ' "$off" "$dim" "$off"
  printf '  %s%s%s\n' "$accent" '██▄▄██  ██▄▄▄▄   ▀██▀   ██▄▄██  ██    ██▄▄' "$off"
  printf '  %s%s%s\n\n' "$accent" '                        ██' "$off"
}

# What it is about to do, given the version already here: empty when there
# is none, `?` when there is one whose version cannot be read.
header() {
  unopen
  case "$1" in
    "" | "$version") title="Installing devpit $version" ;;
    "?") title="Updating devpit to $version" ;;
    *) title="Updating devpit $1 $arrow $version" ;;
  esac
  printf '  %s%s%s\n  %s%s%s\n\n' "$bold" "$title" "$off" "$dim" "$where" "$off"
}

# The version installed now, as `header` takes it.
installed() {
  case "$how" in
    deb) dpkg-query -W -f='${Version}' devpit 2>/dev/null || true ;;
    appimage) [ ! -e "$HOME/.local/bin/devpit" ] || printf '?' ;;
    macos)
      for apps in /Applications "$HOME/Applications"; do
        defaults read "$apps/devpit.app/Contents/Info" CFBundleShortVersionString 2>/dev/null && return 0
      done
      ;;
  esac
  return 0
}

# The last lines: how to open it, and where the docs are.
finish() {
  [ -n "$rich" ] || return 0
  if [ "$1" = dry ]; then
    printf '  %s%s%s Done\n\n  Nothing was installed: a dry run stops after the checks.\n' "$green" "$tick" "$off"
  else
    case "$how:$PATH:" in
      macos:*) opens="Open devpit from Applications." ;;
      appimage:*":$HOME/.local/bin:"* | deb:*) opens="Open devpit from your apps, or run \`devpit\`." ;;
      *) opens="Open devpit from your apps, or run \`$HOME/.local/bin/devpit\`." ;;
    esac
    printf '\n  %s%s%s\n' "$bold" "$opens" "$off"
    printf '  %sFrom here it keeps itself up to date.%s\n' "$dim" "$off"
  fi
  printf '  %sDocs: %s%s\n\n' "$dim" "$DOCS" "$off"
}

fetch() {
  # HTTPS only, and never a downgrade to anything older than TLS 1.2.
  curl --proto '=https' --tlsv1.2 -fsSL "$@"
}

# The big download shows its progress where someone is watching: a hundred
# megabytes in silence reads as a hang.
fetch_big() {
  if [ -n "$rich" ]; then
    fetch_shown "$1" "$2"
  elif [ -t 2 ]; then
    curl --proto '=https' --tlsv1.2 -fL --progress-bar -o "$1" "$2"
  else
    fetch -o "$1" "$2"
  fi
}

# curl in the foreground, so Ctrl-C reaches it, and the bar drawn beside it
# from the headers and the file as they arrive.
fetch_shown() {
  : >"$1.head"
  printf '%s' "$hide"
  meter "$1" &
  meter=$!
  rc=0
  curl --proto '=https' --tlsv1.2 -fsSL -D "$1.head" -o "$1" "$2" 2>"$1.err" || rc=$?
  stop_meter
  if [ "$rc" -ne 0 ]; then
    said=$(cat "$1.err")
    return "$rc"
  fi
  # The finished bar stays, full, above the step it belongs to.
  got=$(size_of "$1")
  bar "${got:-0}" "${got:-0}"
  printf '\n'
  open=''
}

stop_meter() {
  [ -n "${meter:-}" ] || return 0
  kill "$meter" 2>/dev/null || true
  wait "$meter" 2>/dev/null || true
  meter=''
  printf '\r%s%s' "$wipe" "$show" 2>/dev/null || true
}

# Redraws the download until it is stopped: a bar once the size is known, a
# spinner until then.
meter() {
  i=0
  while :; do
    # An async list ignores INT and QUIT, so it stops itself once the installer is gone.
    if ! kill -0 "$$" 2>/dev/null; then
      printf '\r%s%s' "$wipe" "$show"
      exit 0
    fi
    total=$(content_length "$1.head")
    have=$(size_of "$1")
    if [ -n "$total" ] && [ "$total" -gt 0 ]; then
      bar "${have:-0}" "$total"
    else
      printf '\r  %s%s%s %s%s  %s MB%s' "$accent" "$(spinner "$i")" "$off" "$now" "$more" "$(mb "${have:-0}")" "$wipe"
    fi
    i=$((i + 1))
    sleep 0.1 2>/dev/null || sleep 1
  done
}

# The last response's Content-Length: the redirects before it carry their own.
content_length() {
  awk '{ sub(/\r$/, ""); line = tolower($0) }
    line ~ /^http\// { code = $2; size = "" }
    line ~ /^content-length:/ { size = $2 }
    END { if (code == 200 && size ~ /^[0-9]+$/) print size }' "$1" 2>/dev/null || true
}

size_of() { { wc -c <"$1"; } 2>/dev/null | tr -d ' '; }

# Megabytes with one decimal, in integers: sh has no floating point.
mb() {
  tenths=$(($1 * 10 / 1048576))
  printf '%d.%d' $((tenths / 10)) $((tenths % 10))
}

bar() {
  pct=$(($1 * 100 / $2))
  [ "$pct" -le 100 ] || pct=100
  # Padded to the total's width, so the bar's end holds still.
  whole=$(mb "$2")
  tail=$(printf '%3d%%  %*s / %s MB' "$pct" "${#whole}" "$(mb "$1")" "$whole")
  # One column short of the edge, so the line never wraps.
  room=$((cols - ${#tail} - 5))
  fill=$((room * pct / 100))
  printf '\r  %s%s%s%s%s%s  %s%s' "$accent" "$(repeat "$full" "$fill")" "$off" "$dim" \
    "$(repeat "$empty" $((room - fill)))" "$off" "$tail" "$wipe"
}

# Repeated by appending: cutting a string of multibyte glyphs is not portable.
repeat() {
  out='' n=$2
  while [ "$n" -gt 0 ]; do
    out="$out$1"
    n=$((n - 1))
  done
  printf '%s' "$out"
}

spinner() {
  if [ -z "$utf" ]; then
    case $(($1 % 4)) in 0) printf '|' ;; 1) printf '/' ;; 2) printf '-' ;; *) printf '\134' ;; esac
    return 0
  fi
  case $(($1 % 10)) in
    0) printf '⠋' ;; 1) printf '⠙' ;; 2) printf '⠹' ;; 3) printf '⠸' ;; 4) printf '⠼' ;;
    5) printf '⠴' ;; 6) printf '⠦' ;; 7) printf '⠧' ;; 8) printf '⠇' ;; *) printf '⠏' ;;
  esac
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

# Which file fits this machine, and how it goes in. Sets `asset`, `how` and
# `where`, the line that says so.
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
        where="Linux $dot $arch $dot .deb via apt"
      else
        asset="devpit_${version}_${image_arch}.AppImage"
        how=appimage
        where="Linux $dot $arch $dot AppImage in ~/.local/bin"
      fi
      ;;
    Darwin)
      # One universal build for both kinds of Mac. The tarball rather than the
      # .dmg: it is the same .app, it unpacks without mounting anything, and it
      # is what devpit's own updater installs.
      asset="devpit.app.tar.gz"
      how=macos
      where="macOS $dot $arch $dot universal .app"
      ;;
    MINGW* | MSYS* | CYGWIN*)
      fail "on Windows, run the installer: https://github.com/$REPO/releases/download/v$version/devpit_${version}_x64-setup.exe — or run this script inside WSL"
      ;;
    *)
      fail "no build for $os — devpit is released for Linux, macOS and Windows"
      ;;
  esac
}

# The signature, where minisign can check it. SHA256SUMS sits in the same
# release as the file, so it catches a broken download but not a replaced
# release; the signature is what only devpit's key can make.
verify_signature() {
  file=$1
  if ! command -v minisign >/dev/null 2>&1; then
    skipped "signature not checked: install minisign to check it as well" "minisign not found, signature skipped"
    return 0
  fi
  step "" "Checking the signature"
  fetch -o "$file.sig.b64" "$base/$asset.sig" || fail "could not download the signature of $asset"
  # The release keeps each signature base64-encoded, as the updater reads it.
  { base64 -d <"$file.sig.b64" || openssl base64 -d -A <"$file.sig.b64"; } >"$file.minisig" 2>/dev/null ||
    fail "could not read the signature of $asset"
  minisign -Vq -P "$PUBKEY" -m "$file" -x "$file.minisig" >/dev/null 2>&1 ||
    fail "$asset is not signed by devpit's release key — refusing to install it"
  passed "signature matches" "Signature matches devpit's release key"
}

install_deb() {
  file=$1
  version=$2
  installed=$(dpkg-query -W -f='${Version}' devpit 2>/dev/null || true)
  if [ "$installed" = "$version" ]; then
    passed "devpit $version is already installed." "devpit $version is already installed"
    return 0
  fi
  # apt reads the package as the unprivileged `_apt` user, which cannot see
  # into a folder only this user can open; without this it warns and falls
  # back to reading it as root.
  chmod 755 "$(dirname "$file")"
  chmod 644 "$file"
  step "" "Installing through apt"
  if [ "$(id -u)" -eq 0 ]; then
    settle
    apt-get install -y "$file"
  else
    command -v sudo >/dev/null 2>&1 ||
      fail "the .deb needs root, and sudo is not here: run this as root, or with DEVPIT_FORMAT=appimage"
    say "installing through apt, which asks for your password"
    sudo apt-get install -y "$file"
  fi
  passed "devpit $version is installed. Open it from your applications." "Installed through apt"
  if [ -n "$installed" ]; then
    say "if devpit was open, close it and open it again to run $version."
  fi
}

install_appimage() {
  file=$1
  version=$2
  bin="$HOME/.local/bin"
  step "" "Installing"
  mkdir -p "$bin"
  # Under one name that does not change, so devpit replacing itself on update
  # replaces this same file rather than leaving versions to pile up.
  mv "$file" "$bin/devpit"
  chmod 755 "$bin/devpit"
  passed "devpit $version is at $bin/devpit" "Installed at $bin/devpit"
  launcher "$bin/devpit"
  # `ldconfig` lives in /sbin, which is not on every user's PATH.
  if ! { ldconfig -p 2>/dev/null || /sbin/ldconfig -p 2>/dev/null; } | grep -q 'libfuse\.so\.2'; then
    skipped "an AppImage needs FUSE 2, which this machine does not seem to have." "FUSE 2 not found, and an AppImage needs it"
    say "on Ubuntu and Debian: sudo apt install libfuse2"
  fi
  # On a terminal, the last line says how to run it.
  case ":$PATH:" in
    *":$bin:"*) [ -n "$rich" ] || say "run it with: devpit" ;;
    *) say "$bin is not on your PATH — run it with: $bin/devpit" ;;
  esac
}

# Puts devpit in the application menu, with its icon.
#
# A .deb does this through the package; an AppImage in ~/.local/bin is only a
# file, and without this it opened from a terminal and nowhere else. The entry
# and the icons come out of the AppImage itself — the file whose checksum was
# just checked — so they are the ones this version ships.
#
# `StartupWMClass` is what makes the running window group under this icon in
# a dock rather than appear as a second, anonymous app. Never fatal: a missing
# icon is a cosmetic loss, not a reason to leave devpit half installed.
launcher() {
  image=$1
  share="${XDG_DATA_HOME:-$HOME/.local/share}"
  (cd "$work" && "$image" --appimage-extract 'usr/share/icons/*' >/dev/null 2>&1) || true
  if [ -d "$work/squashfs-root/usr/share/icons/hicolor" ]; then
    for icon in "$work"/squashfs-root/usr/share/icons/hicolor/*/apps/devpit-desktop.png; do
      [ -f "$icon" ] || continue
      size=$(basename "$(dirname "$(dirname "$icon")")")
      mkdir -p "$share/icons/hicolor/$size/apps"
      cp "$icon" "$share/icons/hicolor/$size/apps/devpit-desktop.png"
    done
    command -v gtk-update-icon-cache >/dev/null 2>&1 &&
      gtk-update-icon-cache -q -t "$share/icons/hicolor" 2>/dev/null || true
  fi
  mkdir -p "$share/applications"
  cat >"$share/applications/devpit.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=devpit
Exec="$image"
Icon=devpit-desktop
StartupWMClass=devpit-desktop
Terminal=false
Categories=Development;
EOF
  command -v update-desktop-database >/dev/null 2>&1 &&
    update-desktop-database -q "$share/applications" 2>/dev/null || true
  passed "devpit is in your application menu" "Added to the application menu"
}

install_macos() {
  file=$1
  version=$2
  dir=$(dirname "$file")
  step "" "Installing"
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
    passed "devpit $version is already installed." "devpit $version is already installed"
    return 0
  fi
  # Only this exact path, and only once the new copy is known to be there.
  rm -rf "$target"
  mv "$dir/devpit.app" "$target"
  passed "devpit $version is in $apps" "Installed in $apps"
  say "the app is not notarised; if macOS refuses to open it, right-click it and choose Open."
}

main() {
  ui
  command -v curl >/dev/null 2>&1 || fail "curl is needed to download devpit"
  banner

  version=${DEVPIT_VERSION:-}
  if [ -z "$version" ]; then
    step "" "Finding the latest release"
    version=$(latest_version) || fail "could not find the latest release on GitHub"
    found="Latest release ${version#v}"
  else
    found="Release ${version#v}, from DEVPIT_VERSION"
  fi
  version=${version#v}
  # It goes into a URL: digits and dots, and nothing else.
  case "$version" in
    "" | *[!0-9.]* | .* | *.) fail "'$version' is not a version: DEVPIT_VERSION takes one like 0.1.33" ;;
  esac

  choose "$version"
  if [ -n "$rich" ]; then
    header "$(installed)"
  fi
  passed "" "$found"
  if grep -qi microsoft /proc/version 2>/dev/null; then
    say "in WSL devpit opens through WSLg: Windows 11, or Windows 10 with WSLg"
  fi

  work=$(mktemp -d)
  # The folder goes on the way out, however that is. INT and TERM exit rather
  # than clean up and carry on: a trap that only tidies would leave Ctrl-C
  # tidying and then installing anyway.
  trap 'stop_meter; rm -rf "$work"' EXIT
  trap 'exit 129' HUP
  trap 'exit 130' INT
  trap 'exit 131' QUIT
  trap 'exit 143' TERM

  base="https://github.com/$REPO/releases/download/v$version"
  step "downloading $asset ($version)" "Downloading $asset"
  fetch_big "$work/$asset" "$base/$asset" || fail "could not download $base/$asset"
  passed "" "Downloaded $asset"

  step "" "Checking the checksum"
  fetch -o "$work/SHA256SUMS" "$base/SHA256SUMS" || fail "could not download the checksums for $version"
  expected=$(awk -v name="$asset" '$2 == name { print $1; exit }' "$work/SHA256SUMS")
  [ -n "$expected" ] || fail "$asset is not in the release's SHA256SUMS, so it cannot be checked — refusing to install it"
  actual=$(sha256_of "$work/$asset")
  [ "$actual" = "$expected" ] || fail "$asset does not match its checksum — refusing to install it"
  passed "checksum matches" "Checksum matches SHA256SUMS"
  verify_signature "$work/$asset"

  if [ -n "${DEVPIT_DRY_RUN:-}" ]; then
    skipped "dry run: would install $asset as $how, and stop here" "Dry run: would install $asset as $how"
    finish dry
    return 0
  fi

  case "$how" in
    deb) install_deb "$work/$asset" "$version" ;;
    appimage) install_appimage "$work/$asset" "$version" ;;
    macos) install_macos "$work/$asset" "$version" ;;
  esac

  passed "from here devpit keeps itself up to date: it checks when it opens and once a day." "Done"
  finish installed
}

main "$@"
