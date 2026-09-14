import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Skill } from '../gen/bindings'
import { SkillsPane } from './SkillsPane'

afterEach(cleanup)

const listed = vi.fn()
const read = vi.fn()

/* The name is on the row and again as the heading of the skill that is open,
   which is the point of the layout — so a row is asked for as a row. */
const row = (name: string): Element | undefined =>
  Array.from(document.querySelectorAll('.skrow__n')).find((one) => one.textContent === name)

const skill = (over: Partial<Skill> & { name: string }): Skill => ({
  source: 'omc',
  path: `/home/me/.claude/plugins/cache/omc/skills/${over.name}/SKILL.md`,
  description: 'What it does, in the author’s own words.',
  ...over,
})

let catalogue: Skill[] = []

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    skillsList: () => {
      listed()
      return {
        skills: [...catalogue],
        problem: catalogue.length === 0 ? 'no skills directory on this machine' : null,
        directory: '/home/me/.claude-claudin',
      }
    },
    skillsRead: (name: string) => {
      read(name)
      return { name, path: 'x', body: '# Heading\n\nThe instructions themselves.\n' }
    },
    pathOpen: () => null,
    pathReveal: () => null,
  },
}))

vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn() }) }))

beforeEach(() => {
  listed.mockClear()
  read.mockClear()
  catalogue = [skill({ name: 'tdd' }), skill({ name: 'ai-slop-cleaner', source: 'claude' })]
})

/*
 * The column on the right held one sentence and three buttons while the
 * stylesheet carried rules for a rendered SKILL.md that was never fetched.
 */

describe('the skills panel', () => {
  it('shows the body of the skill, not only its name', async () => {
    render(<SkillsPane />)
    expect(await screen.findByText('The instructions themselves.')).toBeTruthy()
  })

  it('reads the skill that was picked', async () => {
    render(<SkillsPane />)
    await screen.findByText('The instructions themselves.')
    fireEvent.click(screen.getByText('ai-slop-cleaner'))
    await waitFor(() => expect(read).toHaveBeenCalledWith('ai-slop-cleaner'))
  })

  it('names the directory it read, because it is not always ~/.claude', async () => {
    // Two installations of the same CLI hold different sets. A list from the
    // wrong one raises nothing and looks exactly right.
    render(<SkillsPane />)
    expect(await screen.findByText('/home/me/.claude-claudin')).toBeTruthy()
  })

  it('re-reads on demand, because a skill arrives from a terminal', async () => {
    render(<SkillsPane />)
    await waitFor(() => expect(row('tdd')).toBeTruthy())
    catalogue = [...catalogue, skill({ name: 'brand-new' })]
    fireEvent.click(screen.getByLabelText('Refresh skills'))
    await waitFor(() => expect(row('brand-new')).toBeTruthy())
    expect(listed).toHaveBeenCalledTimes(2)
  })

  it('counts what is shown against what is installed while filtering', async () => {
    render(<SkillsPane />)
    await waitFor(() => expect(row('tdd')).toBeTruthy())
    fireEvent.change(screen.getByLabelText('Search skills'), { target: { value: 'tdd' } })
    expect(await screen.findByText('1 of 2')).toBeTruthy()
  })

  it('says so when nothing matches instead of showing an empty column', async () => {
    render(<SkillsPane />)
    await waitFor(() => expect(row('tdd')).toBeTruthy())
    fireEvent.change(screen.getByLabelText('Search skills'), { target: { value: 'zzz' } })
    expect(await screen.findByText('No skill matches that.')).toBeTruthy()
  })

  it('says why when the machine has none, once', async () => {
    catalogue = []
    render(<SkillsPane />)
    // Said by the list. The column beside it says "No skill picked." rather
    // than repeating the sentence, which would read as two faults.
    expect(await screen.findByText('no skills directory on this machine')).toBeTruthy()
    expect(screen.getByText('No skill picked.')).toBeTruthy()
  })
})
