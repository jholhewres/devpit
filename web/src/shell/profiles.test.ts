import { describe, expect, it } from 'vitest'

import type { Profile } from '../gen/bindings'
import type { Draft } from './profiles'
import {
  argsOf,
  argsText,
  blank,
  declaredFrom,
  draftOf,
  masked,
  ordered,
  ready,
  secret,
  withVar,
} from './profiles'

const profile = (over: Partial<Profile>): Profile => ({
  id: 'x',
  label: 'X',
  command: 'x',
  driver: '',
  path: null,
  reach: 'runnable',
  base: '',
  args: [],
  env: [],
  mine: false,
  models: [],
  efforts: [],
  effortDefault: null,
  ...over,
})

describe('arguments as a line', () => {
  it('reads the flags a real profile carried', () => {
    expect(argsOf('--permission-mode bypassPermissions')).toEqual([
      '--permission-mode',
      'bypassPermissions',
    ])
  })

  it('survives extra spacing', () => {
    expect(argsOf('  --a   --b  ')).toEqual(['--a', '--b'])
  })

  it('reads nothing as no arguments', () => {
    expect(argsOf('   ')).toEqual([])
  })

  it('goes back to the line it came from', () => {
    const line = '--permission-mode bypassPermissions'
    expect(argsText(argsOf(line))).toBe(line)
  })

  it('cannot produce an argument with a space in it', () => {
    // Splitting on whitespace is what makes the backend rule unreachable from
    // this field, rather than a refusal somebody has to discover.
    expect(argsOf('a b c').every((one) => !/\s/.test(one))).toBe(true)
  })
})

describe('what order the rows come in', () => {
  it('puts the profiles somebody made above the ones devpit noticed', () => {
    const all = [
      profile({ id: 'a', label: 'Aardvark', mine: false }),
      profile({ id: 'z', label: 'Zebra', mine: true }),
    ]
    expect(ordered(all).map((one) => one.id)).toEqual(['z', 'a'])
  })

  it('sorts by name inside each half', () => {
    const all = [
      profile({ id: '2', label: 'Beta', mine: true }),
      profile({ id: '1', label: 'Alpha', mine: true }),
    ]
    expect(ordered(all).map((one) => one.id)).toEqual(['1', '2'])
  })

  it('leaves the list it was given alone', () => {
    const all = [profile({ id: 'a', mine: false }), profile({ id: 'z', mine: true })]
    ordered(all)
    expect(all.map((one) => one.id)).toEqual(['a', 'z'])
  })
})

describe('whether a draft can be saved', () => {
  const draft = (over: Partial<Draft>): Draft => ({ ...blank('claude'), ...over })

  it('needs a name', () => {
    expect(ready(draft({ label: '   ' }))).toBe(false)
    expect(ready(draft({ label: 'GLM' }))).toBe(true)
  })

  it('needs a base agent', () => {
    expect(ready(draft({ label: 'GLM', base: '' }))).toBe(false)
  })

  it('does not need a program, because the base lends one', () => {
    expect(ready(draft({ label: 'GLM', command: '' }))).toBe(true)
  })
})

describe('values that should not be read over a shoulder', () => {
  it('hides a value without hiding that there is one', () => {
    expect(masked('sk-abc123')).not.toContain('abc')
    expect(masked('sk-abc123').length).toBeGreaterThan(0)
  })

  it('shows nothing for nothing', () => {
    expect(masked('')).toBe('')
  })

  it('does not leak the length of a long secret', () => {
    expect(masked('x'.repeat(500)).length).toBe(24)
  })

  it('knows which names carry secrets', () => {
    expect(secret('ANTHROPIC_AUTH_TOKEN')).toBe(true)
    expect(secret('OPENAI_API_KEY')).toBe(true)
    expect(secret('CLAUDE_CONFIG_DIR')).toBe(false)
    expect(secret('ANTHROPIC_BASE_URL')).toBe(false)
  })
})

describe('editing the variable list', () => {
  const env = [
    { name: 'A', value: '1' },
    { name: 'B', value: '2' },
  ]

  it('changes one in place', () => {
    expect(withVar(env, 1, { name: 'B', value: 'two' })).toEqual([
      { name: 'A', value: '1' },
      { name: 'B', value: 'two' },
    ])
  })

  it('drops one', () => {
    expect(withVar(env, 0, null)).toEqual([{ name: 'B', value: '2' }])
  })

  it('leaves the list it was given alone', () => {
    withVar(env, 0, null)
    expect(env).toHaveLength(2)
  })
})

describe('the boundary between a form and the wire', () => {
  it('fills in the fields an older stored profile may not have had', () => {
    // `Declared` makes them optional on purpose, so a profile written before a
    // field existed still parses. The form does not want three maybes.
    const bare = profile({ id: 'a', base: undefined, args: undefined, env: undefined })
    const open = draftOf(bare)
    expect(open.args).toEqual([])
    expect(open.env).toEqual([])
    expect(open.base).toBe('')
  })

  it('trims what somebody typed on the way out', () => {
    const out = declaredFrom({ ...blank('claude'), label: '  GLM  ', command: ' claude ' })
    expect(out.label).toBe('GLM')
    expect(out.command).toBe('claude')
  })

  it('keeps the id, so a rename is not a new profile', () => {
    // The whole reason the id is minted rather than derived from the name.
    const out = declaredFrom({ ...blank('claude'), id: '01J', label: 'Renamed' })
    expect(out.id).toBe('01J')
  })
})
