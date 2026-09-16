# Security

## Reporting a vulnerability

Please report privately through GitHub's
[security advisory form](https://github.com/jholhewres/devpit/security/advisories/new)
rather than a public issue.

Include what you did, what happened, and what you expected. A proof of concept
helps; a working exploit is not required. Expect an acknowledgement within a
week.

## What this software touches

Worth knowing before you install it, and worth keeping in mind when reviewing:

- It **runs arbitrary commands** on your machine — that is its purpose. Any
  path that lets untrusted input reach a command line is a vulnerability.
- It **reads files** from the projects you register. Reads are resolved through
  symlinks and then checked for containment inside the registered root; a path
  that escapes that check is a vulnerability even if the link itself sits
  inside the root.
- It **holds credentials as files**. `~/.devpit` is created `0700` and the
  files that carry secrets are written `0600` — the account token, the hook
  secret, the agent profiles, the settings devpit writes for the CLI. They are
  cleartext: the OS keyring is where they should be and devpit does not use it
  yet. A secret written anywhere else, or written world-readable, is a
  vulnerability.
- It **runs agents under the account you chose**, and a profile's environment
  reaches the CLI on the command line devpit types into the terminal — both
  when a session is launched and when one is attached. A token in a profile is
  therefore in that terminal's scrollback, where anything that can read your
  tmux server can read it.
- It **listens on loopback only**, on a port it writes to `~/.devpit`. Every
  request carries a 32-byte secret from the same directory and is answered 401
  without it. That secret keeps *other users of this machine* out of the hook
  endpoint. It does not defend against code running as you — code running as
  you can read the file.
- It **reads the Claude CLI's own credentials** to show what your plan has
  left, and sends them to `api.anthropic.com` and nowhere else.
- It **captures command output** and stores it as it came. There is no
  redaction, and there is no promise of any: a token printed by a command you
  ran is on the card.
- It **verifies what it installs**. An update is checked against the release
  signing key before its bytes are written anywhere, and a `.deb` is checked
  again when the install command is shown. An update that installs without a
  signature check is a vulnerability.
- The window **runs under a Content-Security-Policy** that allows no remote
  script and no remote frame. A policy loosened enough to let a page fetch
  code is a vulnerability.

## Scope

In scope: this repository. Out of scope: findings that require an attacker to
already have local code execution as your user, since at that point they can
run the commands this app runs anyway.
