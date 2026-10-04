import { describe, expect, it } from 'vitest'

import { filled, RECIPES } from './recipes'
import { stepConfig } from './stepConfig'

describe('a recipe', () => {
  it('fills the Tests step with the command the project runs its tests with', () => {
    const made = filled('tests', 'make test')
    expect(made.kind).toBe('command')
    expect(JSON.parse(stepConfig(made.kind, made.fields, made.base))).toMatchObject({ command: 'make test' })
  })

  it('makes a review that files findings and sends a blocker back', () => {
    const made = filled('review', null)
    const config = JSON.parse(stepConfig(made.kind, made.fields, made.base))
    expect(config).toMatchObject({ verdictField: 'verdict', sendsBackWhen: 'blocker' })
    expect(JSON.parse(config.schema ?? config.expects).properties.findings).toBeDefined()
  })

  it('proves a bug on the base and on the checkout, through variables only', () => {
    const command = filled('prove', null).fields.command!
    expect(command).toContain('"$DEVPIT_BASE_REF"')
    expect(command).toContain('not proven')
    expect(command).not.toContain('{{')
  })

  it('is one of three', () => {
    expect(RECIPES.map((one) => one.id)).toEqual(['tests', 'review', 'prove'])
  })
})
