import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { added, removed, withSkills } from './pills'
import { SkillPills } from './SkillPills'

afterEach(cleanup)

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: { skillsList: () => ({ skills: [{ name: 'tdd' }, { name: 'ai-slop-cleaner' }], problem: null, directory: '/home/me/.claude' }) },
}))

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
    render(<SkillPills picked={[]} onChange={onChange} />)
    fireEvent.click(screen.getByText('+ Skill'))
    fireEvent.click(await screen.findByRole('option', { name: 'tdd' }))
    expect(onChange).toHaveBeenCalledWith(['tdd'])
  })

  it('removes a pill on click', () => {
    const onChange = vi.fn()
    render(<SkillPills picked={['tdd', 'ai-slop-cleaner']} onChange={onChange} />)
    fireEvent.click(screen.getByText('/tdd ✕'))
    expect(onChange).toHaveBeenCalledWith(['ai-slop-cleaner'])
  })
})
