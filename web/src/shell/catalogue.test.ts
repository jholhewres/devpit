import { describe, expect, it } from 'vitest'

import type { KnownAgent, Profile } from '../gen/bindings'
import { catalogue, choosable, defaultLost, detected } from './catalogue'

const agent = (id: string, over: Partial<KnownAgent> = {}): KnownAgent => ({
  id,
  label: id,
  launch: id,
  installed: true,
  enabled: true,
  ...over,
})

const profile = (id: string, over: Partial<Profile> = {}): Profile => ({
  id,
  label: id,
  command: 'claude',
  driver: 'claude',
  path: '/usr/bin/claude',
  reach: 'runnable',
  base: 'claude',
  args: [],
  env: [],
  mine: true,
  models: [],
  efforts: [],
  effortDefault: null,
  ...over,
})

const none = { defaultId: '' }

describe('one list from two sources', () => {
  it('puts what this machine can start above what it cannot', () => {
    // The list answers "what can I start", and an agent that is not here
    // cannot be started however well it sorts.
    const made = catalogue(
      [agent('zeta'), agent('alpha', { installed: false })],
      [],
      none,
    )
    expect(made.map((one) => one.id)).toEqual(['zeta', 'alpha'])
  })

  it('puts the profiles somebody wrote above the ones devpit knows', () => {
    const made = catalogue([agent('claude')], [profile('01JGLM', { label: 'GLM' })], none)
    expect(made.map((one) => one.label)).toEqual(['GLM', 'claude'])
  })

  it('lists a profile once, not twice', () => {
    // `agents.known` already puts declared profiles at the front of its own
    // answer; drawing both is the same row twice with different buttons.
    const made = catalogue([agent('01JGLM', { label: 'GLM' })], [profile('01JGLM')], none)
    expect(made.filter((one) => one.id === '01JGLM')).toHaveLength(1)
  })

  it('says which one is the default', () => {
    const made = catalogue([agent('claude'), agent('codex')], [], { defaultId: 'codex' })
    expect(made.find((one) => one.isDefault)?.id).toBe('codex')
  })

  it('does not move the default to the top', () => {
    // A row that moves when you pick it is a row you then have to find again.
    const made = catalogue([agent('alpha'), agent('zeta')], [], { defaultId: 'zeta' })
    expect(made.map((one) => one.id)).toEqual(['alpha', 'zeta'])
  })

  it('marks what the person can edit', () => {
    const made = catalogue([agent('claude')], [profile('01JGLM')], none)
    expect(made.find((one) => one.id === '01JGLM')?.mine).toBe(true)
    expect(made.find((one) => one.id === 'claude')?.mine).toBe(false)
  })

  it('reads a profile nothing can start as not installed', () => {
    const made = catalogue([], [profile('01J', { reach: 'missing' })], none)
    expect(made[0]!.installed).toBe(false)
  })
})

describe('how many are here', () => {
  it('counts only what can be started', () => {
    expect(detected(catalogue([agent('a'), agent('b', { installed: false })], [], none))).toBe(1)
  })
})

describe('what the default picker offers', () => {
  const entries = catalogue(
    [agent('here'), agent('gone', { installed: false }), agent('off', { enabled: false })],
    [],
    none,
  )

  it('leaves out what cannot be started', () => {
    expect(choosable(entries).map((one) => one.id)).not.toContain('gone')
  })

  it('leaves out what was switched off', () => {
    // A menu whose default is not in it is a menu that opens nothing.
    expect(choosable(entries).map((one) => one.id)).not.toContain('off')
  })

  it('says when the stored default is no longer one of them', () => {
    expect(defaultLost(entries, 'gone')).toBe(true)
    expect(defaultLost(entries, 'here')).toBe(false)
  })

  it('does not call an empty default lost, because none is an answer', () => {
    // A plain shell is a deliberate choice, not a missing one.
    expect(defaultLost(entries, '')).toBe(false)
  })
})

describe('an answer that predates the field', () => {
  it('reads a missing `enabled` as offered', () => {
    // Optional on the wire. An older answer that does not carry it must not
    // hide every agent on the list.
    const older = { id: 'claude', label: 'Claude', launch: 'claude', installed: true } as KnownAgent
    expect(catalogue([older], [], none)[0]!.enabled).toBe(true)
  })

  it('still honours an explicit no', () => {
    expect(catalogue([agent('x', { enabled: false })], [], none)[0]!.enabled).toBe(false)
  })
})

describe('the filter the menus apply', () => {
  it('keeps out what was switched off', async () => {
    const { offered } = await import('./useKnownAgents')
    const list = [agent('on'), agent('off', { enabled: false })]
    expect(offered(list).map((one) => one.id)).toEqual(['on'])
  })

  it('keeps an agent whose answer predates the field', async () => {
    const { offered } = await import('./useKnownAgents')
    const older = { id: 'claude', label: 'Claude', launch: 'claude', installed: true } as KnownAgent
    expect(offered([older])).toHaveLength(1)
  })
})

describe('what is here and what could be', () => {
  const mixed = catalogue(
    [agent('here'), agent('gone', { installed: false })],
    [],
    none,
  )

  it('keeps them apart, because a heading must not argue with its rows', async () => {
    // `Installed` with `not on this machine` under it is exactly that.
    const { here: installed, elsewhere } = await import('./catalogue')
    expect(installed(mixed).map((one) => one.id)).toEqual(['here'])
    expect(elsewhere(mixed).map((one) => one.id)).toEqual(['gone'])
  })

  it('accounts for every row exactly once', async () => {
    const { here: installed, elsewhere } = await import('./catalogue')
    expect(installed(mixed).length + elsewhere(mixed).length).toBe(mixed.length)
  })
})
