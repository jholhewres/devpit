import { describe, expect, it } from 'vitest'

import type { DevflowConfig } from '../gen/bindings'
import { applied, customSet, rubricsWith } from './devflowEdit'

const config: DevflowConfig = {
  enabled: false,
  stages: ['briefing', 'plan', 'execution'].map((name) => ({
    name,
    enabled: true,
    approve: name === 'execution' ? null : 'person',
    checks: name === 'execution' ? [{ name: 'unit', run: 'make test', report: 'none', evidence: [], source: 'Makefile:1' }] : [],
    gate: null,
    rubric: name === 'plan' ? 'our-plan' : null,
    skill: null,
  })),
  reviewers: [],
  blockOn: ['critical'],
  consolidate: 'jev',
  decisionsUse: false,
  decisionsMode: 'shadow',
}

describe("Devflow's edits", () => {
  it('folds an edit into a setup that is not written yet, keeping where a check was found', () => {
    const off = applied(config, 'stages.plan.enabled', false)
    expect(off.stages.find((one) => one.name === 'plan')?.enabled).toBe(false)
    const renamed = applied(off, 'stages.execution.checks', [{ name: 'tests', run: 'make test', report: 'junit' }])
    expect(renamed.stages.find((one) => one.name === 'execution')?.checks).toEqual([
      { name: 'tests', run: 'make test', report: 'junit', evidence: [], source: 'Makefile:1' },
    ])
    expect(applied(config, 'review.reviewers', [{ agent: 'codex', persona: 'devflow-spec' }]).reviewers).toEqual([
      { agent: 'codex', persona: 'devflow-spec', model: null },
    ])
  })

  it('says every choice of a Custom setup as a dotted path', () => {
    const set = customSet(applied(config, 'decisions.use', true))
    expect(set).toMatchObject({
      'stages.briefing.enabled': true,
      'stages.plan.approve': 'person',
      'stages.execution.checks': [{ name: 'unit', run: 'make test', report: 'none' }],
      'review.block_on': ['critical'],
      'decisions.use': true,
    })
    expect(set).not.toHaveProperty('stages.execution.approve')
  })

  it("changes one stage's rubric and drops an emptied one", () => {
    expect(rubricsWith(config, 'briefing', 'our-brief')).toEqual({ briefing: 'our-brief', plan: 'our-plan' })
    expect(rubricsWith(config, 'plan', '  ')).toEqual({})
  })
})
