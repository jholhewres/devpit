import { describe, expect, it } from 'vitest'

import type { IslandSession } from '../gen/bindings'
import {
  CHEERS_FOR,
  DOZES_AFTER,
  applied,
  askedWords,
  headline,
  holds,
  labelled,
  moodOf,
  moodOfAll,
  nowWords,
  ordered,
  verbOf,
  type Ask,
} from './sessions'

const session = (over: Partial<IslandSession> = {}): IslandSession => ({
  sessionId: 's1',
  paneId: 'leaf_1',
  projectId: 'prj_1',
  project: 'api',
  color: null,
  cardId: null,
  card: null,
  root: null,
  state: 'working',
  steps: [],
  said: null,
  at: 0,
  ...over,
})

const step = (tool: string, target: string | null = null) => ({ tool, target, done: false, failed: false, touch: null })

const question = (over: Partial<Ask> = {}): Ask => ({
  id: 'q1',
  sessionId: 's9',
  tool: 'Bash',
  input: '{"command":"rm -rf build\\nls"}',
  from: 'chat',
  keepable: false,
  project: null,
  ...over,
})

describe('what the island hears', () => {
  it('keeps a session until it is gone', () => {
    const one = applied({}, { was: 'changed', session: session() })
    expect(Object.keys(one)).toEqual(['s1'])
    expect(applied(one, { was: 'gone', sessionId: 's1' })).toEqual({})
  })

  it('puts whoever waits on you first', () => {
    const all = {
      a: session({ sessionId: 'a', state: 'done', at: 30 }),
      b: session({ sessionId: 'b', state: 'working', at: 20 }),
      c: session({ sessionId: 'c', state: 'waiting', at: 10 }),
    }
    expect(ordered(all).map((one) => one.sessionId)).toEqual(['c', 'b', 'a'])
  })
})

describe('the mood', () => {
  it('is thinking while a turn has no step yet, and working once it has', () => {
    expect(moodOf(session(), 0)).toBe('thinking')
    expect(moodOf(session({ steps: [step('Read', 'a.rs')] }), 0)).toBe('working')
  })

  it('cheers a finished turn for a moment, then rests', () => {
    expect(moodOf(session({ state: 'done', at: 0 }), CHEERS_FOR - 1)).toBe('done')
    expect(moodOf(session({ state: 'done', at: 0 }), CHEERS_FOR)).toBe('idle')
  })

  it('is the most urgent of all of them', () => {
    const all = [session({ state: 'working', steps: [step('Bash')] }), session({ state: 'waiting' })]
    expect(moodOfAll(all, [], 0)).toBe('waiting')
    expect(moodOfAll(all, [question()], 0)).toBe('asking')
  })

  it('dozes off when nothing has happened for a while', () => {
    const quiet = [session({ state: 'done', at: 0 })]
    expect(moodOfAll(quiet, [], DOZES_AFTER - 1)).toBe('idle')
    expect(moodOfAll(quiet, [], DOZES_AFTER + 1)).toBe('sleeping')
  })

  it('holds the pill up while somebody waits', () => {
    expect(holds([session({ state: 'waiting' })], [])).toBe(true)
    expect(holds([session()], [question()])).toBe(true)
    expect(holds([session()], [])).toBe(false)
  })
})

describe('the words', () => {
  it('names a step by its verb and what it ran on', () => {
    expect(nowWords(session({ steps: [step('Bash', 'npm test')] }), 0)).toBe('Run npm test')
    expect(nowWords(session({ state: 'waiting' }), 0)).toBe('Waiting on you')
  })

  it('tells apart two sessions with the same name', () => {
    const labels = labelled([session({ sessionId: 'b', at: 2 }), session({ sessionId: 'a', at: 1 })])
    expect(labels.get('a')).toBe('api')
    expect(labels.get('b')).toBe('api 2')
  })

  it('names an MCP tool by its own name', () => {
    expect(verbOf('mcp__devpit__devpit_board')).toBe('devpit board')
  })

  it('counts who waits before who works', () => {
    const all = [session({ state: 'working' }), session({ state: 'waiting' })]
    expect(headline(all, [])).toBe('1 waiting')
    expect(headline([session(), session()], [])).toBe('2 working')
    expect(headline([], [])).toBe('Quiet')
  })

  it('says what a question would run, on one line', () => {
    expect(askedWords(question())).toBe('Run rm -rf build')
    expect(askedWords(question({ input: 'not json', tool: 'Edit' }))).toBe('Edit')
  })
})
