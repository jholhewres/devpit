import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

import { CONTEXT_KEYS, stepConfig, stepFields } from './stepConfig'

/*
 * The cases are shared with `apps/desktop/src/steps/recipe_tests.rs`, which
 * feeds the same configs to the rule that accepts a step and to the code that
 * runs one. It is a single rule with a half on each side of the bridge, and
 * two fixtures would agree for about a week.
 */

type Case = {
  kind: string
  fields: Record<string, string>
  config: Record<string, unknown>
}

const cases: readonly Case[] = (
  JSON.parse(
    readFileSync(resolve(process.cwd(), '../crates/steps/tests/fixtures/step-configs.json'), 'utf8'),
  ) as { cases: Case[] }
).cases

describe('what the step form saves', () => {
  it('has cases to check', () => {
    expect(cases.length).toBeGreaterThan(0)
  })

  for (const one of cases) {
    it(`builds a ${one.kind} config out of ${JSON.stringify(one.fields)}`, () => {
      expect(JSON.parse(stepConfig(one.kind, one.fields))).toEqual(one.config)
    })
  }

  /* The form that edits a step starts from what is stored, so the two have to
     be each other's inverse — otherwise opening a step and saving it again
     changes it. */
  for (const one of cases) {
    it(`reads a ${one.kind} config back into its fields`, () => {
      const filled = Object.fromEntries(Object.entries(one.fields).filter(([, value]) => value !== ''))
      expect(stepFields(one.kind, JSON.stringify(one.config))).toEqual(filled)
    })
  }

  it('says a config it cannot read is not readable, rather than guessing', () => {
    expect(stepFields('command', 'make test')).toBeNull()
    expect(stepFields('command', '[1, 2]')).toBeNull()
    expect(stepFields('command', 'null')).toBeNull()
  })

  /* A timeout is optional, so anything that is not a number of seconds is no
     timeout — never a zero, which would read as "give up at once". */
  it('leaves out a timeout that is not a number of seconds', () => {
    expect(JSON.parse(stepConfig('command', { command: 'make test', timeoutSeconds: 'soon' }))).toEqual({
      command: 'make test',
    })
  })
})

/* The list a step chooses from has to be the list the backend accepts, and
   there is no command that hands it over — so it is read off the source. */
describe('the context a step can ask for', () => {
  it('offers exactly the keys the backend answers', () => {
    const rust = readFileSync(
      resolve(process.cwd(), '../apps/desktop/src/steps/context.rs'),
      'utf8',
    )
    const listed = /CONTEXT_KEYS[^=]*=\s*&\[([^\]]*)\]/.exec(rust)?.[1] ?? ''
    const keys = [...listed.matchAll(/"([A-Za-z]+)"/g)].map((one) => one[1])
    expect(keys).toEqual([...CONTEXT_KEYS])
  })
})

/* The form shows a few keys of a config that can hold many. Saving an edit
   rebuilt the config from the form alone and dropped the rest. */
describe('an edited step', () => {
  const stored = JSON.stringify({
    prompt: 'Review the change',
    capUsd: 2,
    model: 'opus',
    verdictField: 'verdict',
    sendsBackWhen: 'fail',
    needsWorktree: true,
    schema: { type: 'object' },
    budgetUsd: 5,
    skills: ['review'],
  })

  it('keeps every key the form does not ask for', () => {
    const saved = JSON.parse(stepConfig('agent', { prompt: 'Review it again', capUsd: '3', model: 'opus' }, stored))
    expect(saved).toEqual({
      prompt: 'Review it again',
      capUsd: 3,
      model: 'opus',
      verdictField: 'verdict',
      sendsBackWhen: 'fail',
      needsWorktree: true,
      schema: { type: 'object' },
      budgetUsd: 5,
      skills: ['review'],
    })
  })

  it('drops a key the form asks for once it has been cleared', () => {
    const saved = JSON.parse(stepConfig('agent', { prompt: 'Review the change', capUsd: '2', model: '' }, stored))
    expect(saved).not.toHaveProperty('model')
    expect(saved.verdictField).toBe('verdict')
    expect(
      JSON.parse(stepConfig('command', { command: 'make test' }, JSON.stringify({ command: 'make', timeoutSeconds: 60, env: { CI: '1' } }))),
    ).toEqual({ command: 'make test', env: { CI: '1' } })
  })

  it('is what the form says, all of it, when the stored config cannot be read', () => {
    expect(JSON.parse(stepConfig('command', { command: 'make test' }, 'make test'))).toEqual({ command: 'make test' })
  })
})
