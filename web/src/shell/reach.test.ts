import { describe, expect, it } from 'vitest'

import type { Profile } from '../gen/bindings'
import { models, told } from './reach'

const profile = (over: Partial<Profile>): Profile => ({
  id: 'x',
  label: 'X',
  command: 'x',
  driver: 'claude',
  path: null,
  reach: 'missing',
  models: [],
  efforts: [],
  effortDefault: null,
  ...over,
})

describe('how far this machine gets', () => {
  it('shows where a runnable command lives', () => {
    const said = told(profile({ reach: 'runnable', path: '/usr/bin/claude' }))
    expect(said.sub).toBe('/usr/bin/claude')
    expect(said.off).toBe(false)
  })

  it('does not call a shell function missing', () => {
    // The bug this replaces: `claudin` is a function in .zshrc with no file,
    // and the row said "not on the PATH" about a command the terminal runs.
    const said = told(profile({ reach: 'shell_only', command: 'claudin' }))
    expect(said.sub).not.toMatch(/not found|PATH/)
    expect(said.off).toBe(false)
  })

  it('says a shell function works in one place and not the other', () => {
    const said = told(profile({ reach: 'shell_only' }))
    expect(said.sub).toMatch(/terminal/)
    expect(said.sub).toMatch(/board/)
  })

  it('gives the three states three different dots', () => {
    const dots = (['runnable', 'shell_only', 'missing'] as const).map(
      (reach) => told(profile({ reach })).dot,
    )
    expect(new Set(dots).size).toBe(3)
  })

  it('dims only what nothing can start', () => {
    expect(told(profile({ reach: 'missing' })).off).toBe(true)
  })

  it('counts models in the singular when there is one', () => {
    expect(models(profile({ models: ['sonnet'] }))).toBe('1 model')
    expect(models(profile({ models: ['sonnet', 'opus'] }))).toBe('2 models')
    expect(models(profile({}))).toBe('')
  })
})
