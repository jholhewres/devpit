import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Profile } from '../gen/bindings'
import { ModelPicker } from './ModelPicker'

afterEach(cleanup)

const openPrefs = vi.fn()
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'prj_1' }, openPrefs }) }))
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null }),
  commands: {
    cliInstallations: () => [
      { directory: '/home/me/.claude', profiles: [], ids: [], default: true },
      { directory: '/home/me/.claude-claudin', profiles: ['claudin'], ids: ['p_claudin'], default: false },
    ],
    skillsList: (directory: string) => ({ skills: directory.endsWith('claudin') ? [{}, {}, {}] : [{}], problem: null, directory }),
    mcpList: (_: string, directory: string) => ({ servers: [{}], sources: [], problem: null, manageWith: '', directory }),
  },
}))

const profile = (over: Partial<Profile>): Profile =>
  ({ id: 'p', label: 'p', command: 'claude', driver: 'claude', path: '/bin/claude', reach: 'runnable', base: 'claude', args: [], env: [], mine: true, models: ['default', 'opus', 'sonnet'], efforts: [], effortDefault: null, ...over }) as Profile

const claude = profile({ id: 'claude', label: 'Claude Code', mine: false, base: '' })
const claudin = profile({
  id: 'p_claudin',
  label: 'claudin',
  env: [{ name: 'CLAUDE_CONFIG_DIR', value: '/home/me/.claude-claudin' }],
})

beforeEach(() => {
  openPrefs.mockClear()
  localStorage.clear()
})

function opened(profiles: Profile[], profileId: string | null, onPick = vi.fn()): typeof onPick {
  render(<ModelPicker profiles={profiles} profileId={profileId} model={null} fixed={null} onPick={onPick} />)
  fireEvent.click(screen.getByLabelText('Account and model'))
  return onPick
}

describe('the model picker', () => {
  /* Two profiles of one CLI wear one mark; the rail has to say which is which. */
  it('names every account on the rail, with what makes it that account', () => {
    opened([claude, claudin], 'claude')
    const rail = screen.getByRole('navigation', { name: 'Accounts' })
    expect(within(rail).getByText('claudin')).toBeTruthy()
    expect(within(rail).getByText('.claude-claudin')).toBeTruthy()
    expect(within(rail).getByText('Default sign-in')).toBeTruthy()
  })

  it('shows versions, what is sent, and which one this chat is on', () => {
    opened([claude], 'claude')
    const current = screen.getByRole('option', { selected: true })
    expect(within(current).getByText("The account's default")).toBeTruthy()
    const opus = screen.getByText('Opus 5.5').closest('[role="option"]') as HTMLElement
    expect(within(opus).getByText('opus')).toBeTruthy()
  })

  it('picks the account and the model together', () => {
    const onPick = opened([claude, claudin], 'claude')
    fireEvent.click(within(screen.getByRole('navigation', { name: 'Accounts' })).getByText('claudin'))
    fireEvent.click(screen.getByText('Sonnet 5'))
    expect(onPick).toHaveBeenCalledWith('p_claudin', 'sonnet')
  })

  it('moves along the rail with the arrows while the search is empty', () => {
    const onPick = opened([claude, claudin], 'claude')
    const field = screen.getByLabelText('Search models')
    fireEvent.keyDown(field, { key: 'ArrowRight' })
    fireEvent.keyDown(field, { key: 'ArrowDown' })
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(onPick).toHaveBeenCalledWith('p_claudin', 'opus')
  })

  /* What changes with the account: where its history, skills and MCP live. */
  it('says where the account on the rail keeps its configuration', async () => {
    opened([claude, claudin], 'claude')
    fireEvent.click(within(screen.getByRole('navigation', { name: 'Accounts' })).getByText('claudin'))
    await waitFor(() => expect(screen.getByText('/home/me/.claude-claudin')).toBeTruthy())
    expect(await screen.findByText('3 skills · 1 MCP server')).toBeTruthy()
  })

  it('points at Providers when there is only one account', () => {
    opened([claude], 'claude')
    expect(screen.getByText(/add it in Settings, Providers/)).toBeTruthy()
    fireEvent.click(screen.getByText('+ Add an account'))
    expect(openPrefs).toHaveBeenCalledWith('providers')
  })
})
