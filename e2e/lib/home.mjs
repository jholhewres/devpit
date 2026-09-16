/*
 * The machine the suite pretends to be on.
 *
 * Everything the app reads — state, git config, the CLI's own directory, the
 * shell's rc files — is under `target/e2e-home`, recreated from nothing on
 * every run. Two reasons, and the second is the one that matters: a suite that
 * reads the real `~/.devpit` tells you about that machine and not about the
 * build, and a suite that *writes* to it has deleted somebody's board.
 */

import { execFileSync } from 'node:child_process'
import { chmodSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

/** A port nothing listens on, so a sign-in attempt fails instead of leaving. */
const CLOSED_PORT = 'http://127.0.0.1:9'

/**
 * The home, wiped and rebuilt. Returns where things are.
 *
 * `name` because test files run in their own processes: two of them seeding
 * the same directory means one wipes the other's git repository halfway
 * through, and the error that comes out is about an ignored file.
 */
export function seedHome(root, name = 'e2e-home') {
  const home = join(root, 'target', name)
  rmSync(home, { recursive: true, force: true })

  for (const dir of [
    '.cache',
    '.config',
    '.local/share',
    '.local/bin',
    '.claude',
    '.devpit',
    'work',
  ]) {
    mkdirSync(join(home, dir), { recursive: true })
  }

  // A login shell reads these, and the app asks a login shell what `claude`
  // is. Empty is not enough: the file has to exist, or zsh falls back to the
  // system one, which is the machine's and not this home's.
  const bin = join(home, '.local/bin')
  const line = `export PATH="${bin}:$PATH"\n`
  for (const rc of ['.zshenv', '.zshrc', '.bashrc', '.profile']) {
    writeFileSync(join(home, rc), line)
  }

  writeFileSync(
    join(home, '.gitconfig'),
    '[user]\n\tname = devpit e2e\n\temail = e2e@devpit.invalid\n[init]\n\tdefaultBranch = main\n',
  )

  const repo = join(home, 'work/fixture')
  mkdirSync(repo, { recursive: true })
  const git = (...args) => execFileSync('git', args, { cwd: repo, env: gitEnv(home) })
  git('init', '-q')
  writeFileSync(join(repo, 'README.md'), '# fixture\n\nA repository with one commit.\n')
  git('add', 'README.md')
  git('commit', '-qm', 'first')

  // The `claude` a login shell in this home finds. A one-line shim rather
  // than a copy: the stub is source in the tree, and a copy would go stale the
  // first time it changed.
  const stub = join(bin, 'claude')
  const script = join(dirname(fileURLToPath(import.meta.url)), '..', 'stub', 'claude.mjs')
  writeFileSync(stub, `#!/bin/sh\nexec "${process.execPath}" "${script}" "$@"\n`)
  chmodSync(stub, 0o755)

  const feed = join(home, 'feed.json')
  writeFileSync(
    feed,
    `${JSON.stringify({ version: '99.0.0', notes: 'A version from a file, for the suite.' }, null, 2)}\n`,
  )

  return { home, repo, feed, bin, stub }
}

/**
 * Another fixture repository in a seeded home, for a test that needs a project
 * of its own — the tests share a home and a driver, and one test archiving the
 * card another one photographs is a flaky suite.
 */
export function seedRepo(home, name) {
  const repo = join(home, 'work', name)
  rmSync(repo, { recursive: true, force: true })
  mkdirSync(repo, { recursive: true })
  const git = (...args) => execFileSync('git', args, { cwd: repo, env: gitEnv(home) })
  git('init', '-q')
  writeFileSync(join(repo, 'README.md'), `# ${name}\n`)
  git('add', 'README.md')
  git('commit', '-qm', 'first')
  return repo
}

function gitEnv(home) {
  return { ...process.env, HOME: home, GIT_CONFIG_GLOBAL: join(home, '.gitconfig') }
}

/**
 * The environment the app is started with.
 *
 * Every `ANTHROPIC_*` and `CLAUDE_*` variable is dropped rather than
 * overridden: the point is a window that has no account anywhere, and a
 * variable left behind is a credential the suite did not know it was using.
 */
export function seedEnv({ home, feed }) {
  const env = {}
  for (const [key, value] of Object.entries(process.env)) {
    if (key.startsWith('ANTHROPIC_') || key.startsWith('CLAUDE_')) continue
    env[key] = value
  }
  return {
    ...env,
    HOME: home,
    XDG_CACHE_HOME: join(home, '.cache'),
    XDG_CONFIG_HOME: join(home, '.config'),
    XDG_DATA_HOME: join(home, '.local/share'),
    PATH: `${join(home, '.local/bin')}:${process.env.PATH ?? ''}`,
    GIT_CONFIG_GLOBAL: join(home, '.gitconfig'),
    DEVPIT_ACCOUNT_ORIGIN: CLOSED_PORT,
    DEVPIT_ACCOUNT_NO_BROWSER: '1',
    DEVPIT_UPDATE_FEED_FILE: feed,
  }
}
