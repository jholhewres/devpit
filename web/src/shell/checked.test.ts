import { describe, expect, it } from 'vitest'

import type { Validity, Verdict, Whose } from '../gen/bindings'
import { CURRENT_MEANS, saidNothing, validityWords, verdictWords, whoseWords } from './checked'

/*
 * The sentences are the feature, so they are what is tested.
 *
 * A screen that says "green" for a command that exited zero is the thing this
 * whole panel exists to stop, and the only place that can go wrong now is the
 * words.
 */

const VERDICTS: Verdict[] = ['passed', 'failed', 'notRun', 'inconclusive']
const VALIDITIES: Validity[] = ['current', 'stale', 'unknown']

describe('what a run proves, in words', () => {
  it('gives every verdict a word, so no state is colour alone', () => {
    for (const verdict of VERDICTS) {
      const said = verdictWords(verdict)
      expect(said.word.length, verdict).toBeGreaterThan(0)
      expect(said.why.length, verdict).toBeGreaterThan(0)
    }
  })

  it('gives every validity a word', () => {
    for (const validity of VALIDITIES) {
      const said = validityWords(validity)
      expect(said.word.length, validity).toBeGreaterThan(0)
      expect(said.why.length, validity).toBeGreaterThan(0)
    }
  })

  /* Sabotage: call `inconclusive` "Passed" and this fails. */
  it('never calls a run with no result a pass', () => {
    expect(verdictWords('inconclusive').word).not.toMatch(/pass/i)
    expect(verdictWords('notRun').word).not.toMatch(/pass/i)
    expect(verdictWords('passed').word).toMatch(/pass/i)
  })

  it('says out loud that exit code zero is not a pass', () => {
    expect(verdictWords('inconclusive').why).toMatch(/exit code zero is not a pass/i)
  })

  /* A result from two commits ago is about other code. Calling it wrong would
     be a different claim, and one this cannot support. */
  it('does not call a stale result wrong', () => {
    expect(validityWords('stale').why).not.toMatch(/wrong|incorrect|invalid/i)
    expect(validityWords('stale').why).toMatch(/moved/i)
  })

  /* The one claim the fingerprint cannot make. */
  it('does not let current mean reproducible', () => {
    expect(CURRENT_MEANS).toMatch(/not the same machine/i)
    for (const validity of VALIDITIES) {
      expect(validityWords(validity).why).not.toMatch(/reproducib|isolat/i)
    }
  })
})

describe('a run with nothing to show', () => {
  const run = { runId: 'run_1', state: 'ok', verdict: 'inconclusive', validity: 'unknown' } as const

  it('is the one that recorded neither a snapshot nor evidence', () => {
    expect(saidNothing({ ...run, ran: null, evidenceVersion: null })).toBe(true)
  })

  it('is not a run that recorded either of them', () => {
    expect(
      saidNothing({
        ...run,
        ran: {
          command: 'make test',
          inDirectory: null,
          declaredEnv: [],
          baseRevision: null,
          headRevision: null,
          inAWorktree: null,
        },
        evidenceVersion: null,
      }),
    ).toBe(false)
    expect(saidNothing({ ...run, ran: null, evidenceVersion: 1 })).toBe(false)
  })
})

describe('who asked for a run and what carried it out', () => {
  const whose = (over: Partial<Whose> = {}): Whose => ({
    asked: 'board',
    askedFrom: null,
    carried: 'process',
    profile: null,
    ...over,
  })

  /* Sabotage: fold the two into one sentence and a test Claude asked for, the
     local runner ran and Codex reviewed reads as one party's work. */
  it('says who asked and what did the work as two sentences', () => {
    const said = whoseWords(whose({ asked: 'checkpoint', carried: 'agent', profile: 'glm' }))
    expect(said.asked).toMatch(/asked for by/i)
    expect(said.asked).toMatch(/checks/i)
    expect(said.carried).toMatch(/carried out by/i)
    expect(said.carried).toMatch(/glm/)
    expect(said.asked).not.toBe(said.carried)
  })

  it('does not invent an origin for a run that recorded none', () => {
    const said = whoseWords(whose({ asked: null, carried: null }))
    expect(said.asked).toMatch(/nobody recorded/i)
    expect(said.carried).toMatch(/nobody recorded/i)
    expect(said.asked).not.toMatch(/board|lane|card/i)
  })

  /* The reference is shown, never resolved: a terminal that has closed must
     not be swapped for another that happens to share its name. */
  it('shows the surface that asked as the reference it is', () => {
    expect(whoseWords(whose({ askedFrom: 'tab_that_is_gone' })).asked).toMatch(/tab_that_is_gone/)
  })

  it('names no account for work a local process did', () => {
    expect(whoseWords(whose({ carried: 'process' })).carried).toMatch(/on this machine/i)
    expect(whoseWords(whose({ carried: 'process' })).carried).not.toMatch(/under /i)
  })

  /* A word a later build invents is not a meaning this one gets to guess. */
  it('does not guess at a surface it does not know', () => {
    expect(whoseWords(whose({ asked: 'from-the-future' })).asked).toMatch(/nobody recorded/i)
  })
})
