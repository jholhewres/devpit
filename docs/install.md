# Installing devpit

The short version is one line, on Linux or macOS: `curl -fsSL https://devpit.app/install.sh | sh`. Everything else is below.

## Requirements

- **Linux**, X11 or Wayland, is the build. macOS has a universal build that is
  not signed or notarised (see [By hand](#by-hand)). Windows has an installer
  that is not code-signed, and the Linux build also runs inside WSL — see
  [Windows](#windows).
- **`tmux`**. Terminals are tmux panes, which is how they outlive the window.
  On Windows it is psmux, and the installer carries it.
- **[Claude Code](https://claude.com/claude-code)** (`claude` on your PATH).
  Chat, agent steps and session steps all run through it; it is the only CLI
  devpit knows how to drive today.

## Installing

One line, on Linux or macOS, and it always takes the latest release:

```sh
curl -fsSL https://devpit.app/install.sh | sh
```

It picks the file that fits the machine, **refuses to install anything that
does not match the release's `SHA256SUMS`** — or, where `minisign` is
installed, is not signed by devpit's release key — and puts it in place: the `.deb`
through apt on Debian and Ubuntu, the AppImage into `~/.local/bin` — with an
entry and an icon in the application menu — on other Linux, and `devpit.app`
into Applications on macOS. Read
[`install.sh`](../install.sh) before piping it into a shell — it is short on
purpose, and the site serves this same file. Run it again at any time to catch up; it says so when there is
nothing to do. On Windows, run the installer instead — see [Windows](#windows).

Three settings, all optional: `DEVPIT_VERSION=0.1.7` installs that version,
`DEVPIT_FORMAT=appimage` takes the AppImage even where apt is available, and
`DEVPIT_DRY_RUN=1` downloads and verifies without installing.

### Staying up to date

Once installed, devpit checks for a new release when it opens and once a day
after, and says so in a card — nothing to run by hand. The switch is in
Settings → General, on unless you turn it off. On Linux and macOS your
terminals keep running through an update: they are tmux sessions, and tmux
does not go down with the window.

- **AppImage** — downloads, verifies the signature, installs over itself and
  restarts.
- **`.deb`** — downloads and verifies, then installs through polkit, which asks
  for your password: devpit never holds root itself. It restarts into the new
  version once the package is in. (Updating *to* 0.1.6 still needed closing and
  reopening devpit by hand; from 0.1.6 on it restarts itself.)
- **macOS** — downloads the `.app`, checks the signature and replaces itself.
- **Windows** — downloads the installer, checks the signature, runs it and
  reopens. **Your terminals close**: Windows will not replace a program that is
  running, so the installer ends the psmux inside devpit first. The card says
  so before you restart.

### By hand

Every file is on the [latest release][releases], and the name carries the
version, so take the current one from that page:

- Linux: `devpit_<version>_amd64.deb` / `_arm64.deb`, or
  `devpit_<version>_amd64.AppImage` / `_aarch64.AppImage`.
- macOS: `devpit_<version>_universal.dmg`, for Apple silicon and Intel alike.
- Windows: `devpit_<version>_x64-setup.exe`.

The macOS build is **not signed with an Apple Developer ID and not
notarised**, so a copy downloaded through a browser is refused by Gatekeeper
the first time with "devpit is damaged" or "cannot be opened". That is the
missing certificate talking, not the file. Open it once with right-click →
Open, or clear the quarantine flag yourself — after checking `SHA256SUMS`:

```sh
xattr -dr com.apple.quarantine /Applications/devpit.app
```

### Windows

Take `devpit_<version>_x64-setup.exe` from the [latest release][releases] and
run it. It is an NSIS installer that installs for your user only — no
administrator rights — with psmux, the tmux devpit's terminals run on there,
inside it. It adds devpit to the Start menu and removes cleanly from Settings →
Apps. It needs Claude Code on PATH (`claude`), and WebView2, which the
installer fetches if Windows lacks it. From then on devpit updates itself (see
[Staying up to date](#staying-up-to-date)).

The installer is **not signed with a code-signing certificate**, so SmartScreen
stops it the first time with "Windows protected your PC". That is the missing
certificate talking, not the file: check it against `SHA256SUMS` first — in
PowerShell, `(Get-FileHash .\devpit_<version>_x64-setup.exe).Hash.ToLower()`
is the line's first half — then More info → Run anyway.

**Or inside WSL.** The Linux build runs in WSL 2, and its window is drawn by
WSLg on the Windows desktop. The terminals are tmux sessions inside WSL and
outlive the window there, updates included.

1. Windows 11, or Windows 10 21H2 or later, with WSL 2 and a Linux
   distribution: `wsl --install` in PowerShell, then restart.
2. In the distribution's shell: `sudo apt install tmux wslu`, and Claude Code
   as its own install page says.
3. The same line as on Linux, inside WSL:
   `curl -fsSL https://devpit.app/install.sh | sh`
4. Keep projects in the Linux file system (`~/…`), not under `/mnt/c`: git and
   the file watchers are many times slower across that border.

`wslu` is what opens a link in the Windows browser. Two things do not reach
across: signed-in sessions brought from a Windows browser, and files dropped
from Explorer.

Every release carries a `SHA256SUMS`, and checking a download is one line:

```sh
sha256sum -c SHA256SUMS --ignore-missing
```

The `.sig` beside each package is the updater's, not a substitute for this: it
is what an installed devpit checks before it replaces itself, against a public
key compiled into the binary you are already running. A first download has no
such binary to check it with, which is what `SHA256SUMS` is for — and what
`install.sh` checks for you.

### Uninstalling, or switching

Removing devpit never removes your work. Projects, cards, conversations and
the terminal sessions live in `~/.devpit`, which belongs to you and not to the
package — so a reinstall, in any format, opens exactly where you left off.

- **`.deb`** — `sudo apt remove devpit`
- **AppImage** — `rm ~/.local/bin/devpit ~/.local/share/applications/devpit.desktop ~/.local/share/icons/hicolor/*/apps/devpit-desktop.png`
- **macOS** — `rm -rf /Applications/devpit.app`
- **Windows** — Settings → Apps → devpit → Uninstall. Your work is in
  `%USERPROFILE%\.devpit`.

To erase everything, remove `~/.devpit` as well — and `~/.devpit-dev`, if you
ever ran a development build.

To switch from the `.deb` to the AppImage, remove the package and run the
installer with `DEVPIT_FORMAT=appimage`; without it, a machine that has apt is
given the `.deb` again:

```sh
sudo apt remove devpit
curl -fsSL https://devpit.app/install.sh | DEVPIT_FORMAT=appimage sh
```

[releases]: https://github.com/jholhewres/devpit/releases/latest
