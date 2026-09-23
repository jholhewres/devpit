import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Profile } from '../gen/bindings'
import { added, removed, withSkills } from './pills'
import { SkillPills } from './SkillPills'

afterEach(cleanup)

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    cliInstallations: () => [
      { directory: '/home/me/.claude', profiles: [], ids: [], default: true },
      { directory: '/home/me/.claude-claudin', profiles: ['claudin'], ids: ['p_claudin'], default: false },
    ],
    skillsList: (directory: string | null) =>
      directory === '/home/me/.claude-claudin'
        ? { skills: [{ name: 'only-on-claudin' }], problem: null, directory }
        : { skills: [{ name: 'tdd' }, { name: 'ai-slop-cleaner' }], problem: null, directory: '/home/me/.claude' },
  },
}))

const claudin = {
  id: 'p_claudin', label: 'claudin', command: 'claude', driver: 'claude', path: '/bin/claude', reach: 'runnable', mine: true,
} as Profile

describe('the prompt a pill makes', () => {
  it('puts one skill on its own first line, where the CLI runs a command', () => {
    expect(withSkills('fix the parser', ['tdd'])).toBe('/tdd\nfix the parser')
  })

  it('names several in a sentence rather than stacking commands', () => {
    expect(withSkills('clean up', ['tdd', 'ai-slop-cleaner', 'review'])).toBe(
      'Use the tdd, ai-slop-cleaner and review skills.\nclean up',
    )
  })

  it('leaves the prompt alone with no skill, and counts a skill once', () => {
    expect(withSkills('hi', [])).toBe('hi')
    expect(withSkills('hi', ['tdd', 'tdd'])).toBe('/tdd\nhi')
    expect(added(['tdd'], 'tdd')).toEqual(['tdd'])
    expect(removed(['tdd', 'x'], 'tdd')).toEqual(['x'])
  })
})

describe('the skill pills', () => {
  it('offers the installed skills and adds the one picked', async () => {
    const onChange = vi.fn()
    render(<SkillPills picked={[]} onChange={onChange} profile={undefined} />)
    fireEvent.click(screen.getByText('+ Skill'))
    fireEvent.click(await screen.findByRole('option', { name: 'tdd' }))
    expect(onChange).toHaveBeenCalledWith(['tdd'])
  })

  /* Each account has its own ~/.claude-*; its skills are the ones it runs. */
  it("offers the skills of the installation the chat's profile runs against", async () => {
    render(<SkillPills picked={[]} onChange={vi.fn()} profile={claudin} />)
    fireEvent.click(screen.getByText('+ Skill'))
    expect(await screen.findByRole('option', { name: 'only-on-claudin' })).toBeTruthy()
    expect(screen.queryByRole('option', { name: 'tdd' })).toBeNull()
  })

  it('removes a pill on click', () => {
    const onChange = vi.fn()
    render(<SkillPills picked={['tdd', 'ai-slop-cleaner']} onChange={onChange} profile={undefined} />)
    fireEvent.click(screen.getByText('/tdd ✕'))
    expect(onChange).toHaveBeenCalledWith(['ai-slop-cleaner'])
  })
})
