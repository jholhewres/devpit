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
- It **will hold credentials**. Secrets belong in the OS keyring, never in the
  SQLite database, never in a config file, and never in a synced payload. A
  secret written to disk in cleartext is a vulnerability.
- It **listens on loopback only**. Anything reachable off the machine without
  an explicit, deliberate opt-in is a vulnerability.
- It **captures command output**. Output is redacted before it is stored.
  A redaction bypass that lets a token reach disk is a vulnerability.

## Scope

In scope: this repository. Out of scope: findings that require an attacker to
already have local code execution as your user, since at that point they can
run the commands this app runs anyway.
