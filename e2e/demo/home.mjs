/*
 * The machine the launch assets pretend to be on.
 *
 * The e2e home, made over for a camera: a person who does not exist, a prompt
 * that shows no host name, the demo agent where `claude` is looked for, and
 * repositories with a little history. devpit's own root is a short link into
 * it, because a tmux socket path is capped near 108 bytes and a checkout under
 * a worktree is already past that.
 */

import { execFileSync, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { seedEnv, seedHome } from '../lib/home.mjs'
import { PERSON, PROJECTS } from './world.mjs'

const here = dirname(fileURLToPath(import.meta.url))

export function demoHome(root) {
  const seeded = seedHome(root, 'demo-home')
  const { home } = seeded

  // A short name per checkout, so two checkouts never share a root.
  const tag = createHash('sha256').update(root).digest('hex').slice(0, 8)
  const devpit = join(process.env.XDG_RUNTIME_DIR || tmpdir(), `devpit-demo-${tag}`)
  spawnSync('tmux', ['-S', join(devpit, 'tmux.sock'), 'kill-server'], { stdio: 'ignore' })
  // Agents a run that was killed left behind post their hooks to whichever
  // devpit listens at this root next, and act in its pictures.
  spawnSync('pkill', ['-f', `${devpit}/hooks.json`], { stdio: 'ignore' })
  rmSync(devpit, { force: true, recursive: true })
  symlinkSync(join(home, '.devpit'), devpit)

  writeFileSync(join(home, '.gitconfig'), `[user]\n\tname = ${PERSON.name}\n\temail = ${PERSON.email}\n[init]\n\tdefaultBranch = main\n`)

  // The working directory, and nothing of the machine, in every prompt.
  const path = `export PATH="${join(home, '.local/bin')}:/usr/local/bin:/usr/bin:/bin"\n`
  writeFileSync(join(home, '.zshenv'), path)
  writeFileSync(join(home, '.zshrc'), `${path}PROMPT='%F{8}%~%f %# '\n`)
  for (const rc of ['.bashrc', '.profile']) writeFileSync(join(home, rc), `${path}PS1='\\w \\$ '\n`)

  // Same shim the e2e writes, pointed at the demo agent.
  writeFileSync(seeded.stub, `#!/bin/sh\nexec "${process.execPath}" "${join(here, 'agent.mjs')}" "$@"\n`)

  // The running version, so nothing offers an update.
  const version = readFileSync(join(root, 'Cargo.toml'), 'utf8').match(/^version = "(.*)"/m)[1]
  writeFileSync(seeded.feed, `${JSON.stringify({ version, notes: '' })}\n`)

  const repos = Object.fromEntries(PROJECTS.map((project) => [project.name, repository(home, project)]))
  return { ...seeded, devpit, repos }
}

/** A project's repository, with a short history on fixed dates. */
function repository(home, project) {
  const where = join(home, 'work', project.name)
  mkdirSync(where, { recursive: true })
  let day = 20
  const git = (...args) => {
    const date = `2026-09-${day}T10:00:00Z`
    execFileSync('git', args, {
      cwd: where,
      env: { ...process.env, HOME: home, GIT_CONFIG_GLOBAL: join(home, '.gitconfig'), GIT_AUTHOR_DATE: date, GIT_COMMITTER_DATE: date },
    })
  }
  const commit = (message, files) => {
    for (const [file, text] of files) {
      mkdirSync(dirname(join(where, file)), { recursive: true })
      writeFileSync(join(where, file), text)
    }
    git('add', '-A')
    git('commit', '-qm', message)
    day += 3
  }
  git('init', '-q')
  const [readme, ...rest] = Object.entries(project.files)
  commit('Initial commit', [readme])
  if (rest.length > 0) commit('Add the service skeleton', rest)
  commit('Run the tests on every push', [['.ci.yml', 'test:\n  run: make test\n']])
  return where
}

/** The app's environment: the e2e one, drawn at 2x, with the short root. */
export function demoEnv(seeded) {
  return { ...seedEnv(seeded), GDK_SCALE: '2', DEVPIT_HOME: seeded.devpit }
}

/** Refuses a window whose state is anywhere but this home. */
export function keptInside(statePath, home) {
  const real = realpathSync(statePath)
  if (!real.startsWith(realpathSync(home))) {
    throw new Error(`the app is keeping state in ${real}, which is not under ${home}`)
  }
}
