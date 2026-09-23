import { describe, expect, it } from 'vitest'

import type { Profile } from '../gen/bindings'
import { accountHint, current, keyOf, rowsFor, SHELF, stepTab } from './modelPick'

const profile = (over: Partial<Profile>): Profile =>
  ({ id: 'p', label: 'p', command: 'claude', driver: 'claude', path: '/bin/claude', reach: 'runnable', base: 'claude', args: [], env: [], mine: true, models: [], efforts: [], effortDefault: null, ...over }) as Profile

const claude = profile({ id: 'claude', label: 'Claude Code', mine: false, base: '', models: ['default', 'opus', 'sonnet'] })
const glm = profile({
  id: 'glm',
  label: 'glm',
  models: ['default', 'glm-5.3[1m]'],
  env: [
    { name: 'ANTHROPIC_BASE_URL', value: 'https://api.z.ai/api/anthropic' },
    { name: 'CLAUDE_CONFIG_DIR', value: '/home/me/.claude-glm' },
  ],
})
const both = [claude, glm]

describe('what the picker lists', () => {
  it('lists one account’s models when its tab is open', () => {
    expect(rowsFor(both, 'glm', '', []).map((row) => row.model)).toEqual(['default', 'glm-5.3[1m]'])
  })

  it('keeps only the starred on the shelf', () => {
    const starred = keyOf({ profile: glm, model: 'glm-5.3[1m]' })
    expect(rowsFor(both, SHELF, '', [starred]).map(keyOf)).toEqual([starred])
  })

  /* A search that stayed inside the tab would hide the model just typed. */
  it('searches every account, by the name shown and by what is sent', () => {
    expect(rowsFor(both, 'glm', 'opus 5.5', []).map(keyOf)).toEqual(['claude:opus'])
    expect(rowsFor(both, 'claude', '[1m]', []).map(keyOf)).toEqual(['glm:glm-5.3[1m]'])
  })

  it('steps along the rail and wraps, the shelf included', () => {
    expect(stepTab(both, SHELF, 1)).toBe('claude')
    expect(stepTab(both, 'glm', 1)).toBe(SHELF)
    expect(stepTab(both, SHELF, -1)).toBe('glm')
  })

  it('treats no pick as the account’s default', () => {
    expect(current(claude, null)).toBe('default')
    expect(current(claude, 'opus')).toBe('opus')
    expect(current(undefined, null)).toBe('')
  })
})

describe('what tells two accounts of one CLI apart', () => {
  it('names the endpoint’s host when it sends elsewhere', () => {
    expect(accountHint(glm)).toBe('api.z.ai')
  })

  it('names the config folder when it signs in elsewhere', () => {
    const claudin = profile({ env: [{ name: 'CLAUDE_CONFIG_DIR', value: '/home/me/.claude-claudin' }] })
    expect(accountHint(claudin)).toBe('.claude-claudin')
  })

  it('says a plain one is the default sign-in, and names an unusual program', () => {
    expect(accountHint(claude)).toBe('Default sign-in')
    expect(accountHint(profile({ command: 'claudin' }))).toBe('claudin')
  })
})
