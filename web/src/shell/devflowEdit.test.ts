import { describe, expect, it } from 'vitest'

import type { DevflowConfig } from '../gen/bindings'
import { added, applied, customSet, moved, removed, rubricsWith, stageJson } from './devflowEdit'

const config: DevflowConfig = {
  enabled: false,
  version: 2,
  teams: [],
  locked: [],
  stagesFrom: 'project',
  stages: ['briefing', 'plan', 'execution'].map((name) => ({
    name,
    uses: `builtin/${name}`,
    own: false,
    publish: null,
    e2eDir: null,
    command: null,
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

  it('keeps the release checks when a stage moves, and folds an edit of them in', () => {
    const release = [{ name: 'e2e', run: 'make e2e', report: 'none', evidence: [], source: null }]
    const withRelease: DevflowConfig = {
      ...config,
      stages: config.stages.map((one) => (one.name === 'execution' ? { ...one, release } : one)),
    }
    const execution = moved(withRelease, 'plan', -1).find((one) => one.uses === 'builtin/execution')
    expect(execution?.release).toEqual([{ name: 'e2e', run: 'make e2e', report: 'none' }])
    expect(moved(config, 'plan', -1).find((one) => one.uses === 'builtin/execution')).not.toHaveProperty('release')
    const edited = applied(withRelease, 'stages.execution.release', [{ name: 'suite', run: 'make suite' }])
    const stage = edited.stages.find((one) => one.name === 'execution')
    expect(stage?.release).toEqual([{ name: 'suite', run: 'make suite', report: 'none', evidence: [], source: null }])
    expect(stage?.checks.map((one) => one.name)).toEqual(['unit'])
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

describe("Devflow's list of stages", () => {
  const own = { ...config.stages[0]!, name: 'pr', uses: 'project/pr', own: true, approve: 'person', command: './scripts/pr.sh' }
  const withOwn: DevflowConfig = { ...config, stages: [...config.stages, own] }

  it('says each stage the way version 2 lists it', () => {
    expect(stageJson(config.stages[2]!)).toEqual({ uses: 'builtin/execution', enabled: true, checks: [{ name: 'unit', run: 'make test', report: 'none' }] })
    expect(stageJson(own)).toEqual({ uses: 'project/pr', enabled: true, approve: 'person', gate: { command: './scripts/pr.sh' } })
    expect(stageJson({ ...config.stages[1]!, unblockOn: 'gate' })).toEqual({ uses: 'builtin/plan', enabled: true, approve: 'person', unblock_on: 'gate' })
  })

  it('moves, removes and adds a stage, keeping the others as they are', () => {
    expect(moved(withOwn, 'pr', -1).map((one) => one.uses)).toEqual(['builtin/briefing', 'builtin/plan', 'project/pr', 'builtin/execution'])
    expect(moved(withOwn, 'briefing', -1).map((one) => one.uses)).toEqual(withOwn.stages.map((one) => one.uses))
    expect(removed(withOwn, 'plan').map((one) => one.uses)).toEqual(['builtin/briefing', 'builtin/execution', 'project/pr'])
    expect(added(config, ' acme/ticket ', '')).toContainEqual({ uses: 'acme/ticket', enabled: true })
    expect(added(config, 'project/pr', 'make pr')).toContainEqual({ uses: 'project/pr', enabled: true, gate: { command: 'make pr' } })
  })
})
