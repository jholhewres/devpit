import { describe, expect, it } from 'vitest'

import { modelHint, modelName, modelsOf, modelsText, movedTo } from './models'

const glm = [
  { name: 'ANTHROPIC_MODEL', value: 'glm-5.3[1m]' },
  { name: 'ANTHROPIC_DEFAULT_OPUS_MODEL', value: 'glm-5.3' },
]

describe('what a model is called on screen', () => {
  /* A fine argument to the CLI, and a label that names no model. */
  it('says whose default it is rather than the word default', () => {
    expect(modelName('default')).toBe('Account default')
  })

  it('names an alias by the version it runs', () => {
    expect(modelName('opus')).toBe('Opus 5.5')
    expect(modelName('sonnet')).toBe('Sonnet 5')
    expect(modelName('haiku')).toBe('Haiku 4.5')
    expect(modelName('fable')).toBe('Fable 5.1')
  })

  it('reads a dated id as family and version', () => {
    expect(modelName('claude-opus-5-5')).toBe('Opus 5.5')
    expect(modelName('claude-sonnet-5')).toBe('Sonnet 5')
    expect(modelName('claude-haiku-4-5-20251001')).toBe('Haiku 4.5')
  })

  it('says when the context window is the wide one', () => {
    expect(modelName('opus[1m]')).toBe('Opus 5.5 · 1M')
    expect(modelName('glm-5.3[1m]')).toBe('glm-5.3 · 1M')
  })

  it('leaves a model it does not know as it was written', () => {
    expect(modelName('glm-4.7')).toBe('glm-4.7')
  })

  /* `glm` points every alias at z.ai: its "opus" is not Opus. */
  it('names an alias a profile moved by where it went', () => {
    expect(modelName('opus', glm)).toBe('glm-5.3')
    expect(modelName('default', glm)).toBe('glm-5.3 · 1M')
    expect(modelName('sonnet', glm)).toBe('Sonnet 5')
    expect(movedTo('haiku', glm)).toBeNull()
  })

  it('shows what is sent, and where a moved alias goes', () => {
    expect(modelHint('opus')).toBe('opus')
    expect(modelHint('opus', glm)).toBe('opus → glm-5.3')
  })
})

describe('a profile’s own models, as typed', () => {
  it('takes commas or lines, once each, in order', () => {
    expect(modelsOf('glm-5.3[1m], glm-4.7\nglm-5.3[1m]\n')).toEqual(['glm-5.3[1m]', 'glm-4.7'])
    expect(modelsOf('  ')).toEqual([])
  })

  it('reads back what it wrote', () => {
    expect(modelsOf(modelsText(['a', 'b[1m]']))).toEqual(['a', 'b[1m]'])
  })
})
