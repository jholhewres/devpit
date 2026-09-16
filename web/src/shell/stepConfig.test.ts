import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

import { CONTEXT_KEYS, stepConfig } from './stepConfig'

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
