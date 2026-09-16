import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { AgentChoice, Declared, KnownAgent, Profile } from '../gen/bindings'
import { ProviderRows } from './ProviderRows'

afterEach(cleanup)

let listed: Profile[] = []
let known: KnownAgent[] = []
let choice: AgentChoice = { defaultId: '', disabled: [], hooks: true }
let refusal: string | null = null
const saved = vi.fn()
const removed = vi.fn()
const defaulted = vi.fn()
const switched = vi.fn()
const hooked = vi.fn()

const opened: string[] = []

vi.mock('./live', () => ({
  ask: (call: () => unknown) =>
    Promise.resolve(
      refusal
        ? { data: null, error: refusal, loading: false }
        : { data: call(), error: null, loading: false },
    ),
  commands: {
    agentProfiles: () => listed,
    urlOpen: (url: string) => (opened.push(url), { path: url }),
    agentsKnown: () => known,
    agentChoice: () => choice,
    agentDefaultSet: (id: string) => (defaulted(id), { ...choice, defaultId: id }),
    agentEnabledSet: (id: string, on: boolean) => (switched(id, on), choice),
    agentHooksSet: (on: boolean) => (hooked(on), { ...choice, hooks: on }),
    agentProfileSave: (declared: Declared) => (saved(declared), listed),
    agentProfileRemove: (id: string) => (removed(id), listed),
  },
}))

const agent = (id: string, over: Partial<KnownAgent> = {}): KnownAgent => ({
  id,
  label: id,
  launch: id,
  installed: true,
  enabled: true,
  ...over,
})

const profile = (id: string, over: Partial<Profile> = {}): Profile => ({
  id,
  label: id,
  command: 'claude',
  driver: 'claude',
  path: '/usr/bin/claude',
  reach: 'runnable',
  base: 'claude',
  args: [],
  env: [],
  mine: true,
  models: [],
  efforts: [],
  effortDefault: null,
  ...over,
})

beforeEach(() => {
  listed = []
  known = [agent('claude', { label: 'Claude Code' })]
  choice = { defaultId: '', disabled: [], hooks: true }
  refusal = null
  for (const spy of [saved, removed, defaulted, switched, hooked]) spy.mockClear()
})

async function shown(): Promise<void> {
  render(<ProviderRows />)
  await waitFor(() => expect(screen.getByText('Default agent')).toBeTruthy())
}

/*
 * Three questions, in the order somebody asks them: what opens when I start a
 * terminal, what is on this machine, and what exactly does each one run. The
 * pane used to answer only the second, as a list of paths.
 */

describe('what opens when I start a terminal', () => {
  it('offers a plain shell as an answer, not as a missing one', async () => {
    await shown()
    fireEvent.click(screen.getByText('No agent'))
    await waitFor(() => expect(defaulted).toHaveBeenCalledWith(''))
  })

  it('offers each agent this machine can start', async () => {
    known = [agent('claude', { label: 'Claude Code' }), agent('codex', { label: 'Codex' })]
    await shown()
    const picker = screen.getByRole('radiogroup', { name: 'Default agent' })
    fireEvent.click(within(picker).getByText('Codex'))
    await waitFor(() => expect(defaulted).toHaveBeenCalledWith('codex'))
  })

  it('says which one is chosen', async () => {
    choice = { defaultId: 'claude', disabled: [], hooks: true }
    await shown()
    const picker = screen.getByRole('radiogroup', { name: 'Default agent' })
    expect(within(picker).getByText('Claude Code').getAttribute('aria-checked')).toBe('true')
    expect(within(picker).getByText('No agent').getAttribute('aria-checked')).toBe('false')
  })

  it('does not offer one that is not here', async () => {
    known = [agent('gone', { label: 'Gone', installed: false })]
    await shown()
    const picker = screen.getByRole('radiogroup', { name: 'Default agent' })
    expect(picker.textContent).not.toContain('Gone')
  })

  it('says when the stored default has gone away', async () => {
    // A default pointing at something absent opens nothing. Said here rather
    // than discovered at the next launch.
    known = [agent('gone', { installed: false })]
    choice = { defaultId: 'gone', disabled: [], hooks: true }
    await shown()
    expect(screen.getByText(/not on this machine any more/)).toBeTruthy()
  })
})

describe('what is on this machine', () => {
  it('counts what can actually be started', async () => {
    known = [agent('a'), agent('b'), agent('c', { installed: false })]
    await shown()
    expect(screen.getByText('2 detected')).toBeTruthy()
  })

  it('shows the line each one would run, not a description of it', async () => {
    known = [agent('claude', { label: 'Claude Code', launch: 'claude --permission-mode x' })]
    await shown()
    expect(screen.getByText('claude --permission-mode x')).toBeTruthy()
  })

  it('does not call a shell function missing', async () => {
    // The terminal runs it every day. A row that said "not found" about it
    // was the bug this whole pane started from.
    listed = [profile('claudin', { label: 'Claudin', reach: 'shell_only', path: null })]
    await shown()
    expect(screen.queryByText(/not found/)).toBeNull()
    expect(screen.getByText(/shell function/)).toBeTruthy()
  })

  it('can switch one off, and out of the menus', async () => {
    await shown()
    fireEvent.click(screen.getAllByText('Disabled')[0]!)
    await waitFor(() => expect(switched).toHaveBeenCalledWith('claude', false))
  })

  it('will not make an absent agent the default', async () => {
    known = [agent('gone', { label: 'Gone', installed: false })]
    await shown()
    expect((screen.getByText('Set default') as HTMLButtonElement).disabled).toBe(true)
  })
})

describe('what exactly does each one run', () => {
  it('opens the same editor for a built-in as for a profile', async () => {
    // An override and an account are the same three fields.
    await shown()
    fireEvent.click(screen.getByLabelText('Show how Claude Code is started'))
    expect(screen.getByLabelText('Program')).toBeTruthy()
    expect(screen.getByLabelText('Arguments')).toBeTruthy()
    expect(screen.getByText('Add variable')).toBeTruthy()
  })

  it('saves an override against the agent it was opened from', async () => {
    await shown()
    fireEvent.click(screen.getByLabelText('Show how Claude Code is started'))
    fireEvent.change(screen.getByLabelText('Arguments'), {
      target: { value: '--permission-mode bypassPermissions' },
    })
    fireEvent.click(screen.getByText('Save'))

    await waitFor(() => expect(saved).toHaveBeenCalled())
    const sent = saved.mock.calls[0]![0] as Declared
    expect(sent.base).toBe('claude')
    expect(sent.args).toEqual(['--permission-mode', 'bypassPermissions'])
  })

  it('keeps the id when a profile is renamed', async () => {
    listed = [profile('01JGLM', { label: 'GLM' })]
    known = []
    await shown()
    fireEvent.click(screen.getByLabelText('Show how GLM is started'))
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'GLM 4.6' } })
    fireEvent.click(screen.getByText('Save'))

    await waitFor(() => expect(saved).toHaveBeenCalled())
    expect((saved.mock.calls[0]![0] as Declared).id).toBe('01JGLM')
  })

  it('only offers to remove what somebody declared', async () => {
    await shown()
    fireEvent.click(screen.getByLabelText('Show how Claude Code is started'))
    expect(screen.queryByText('Remove')).toBeNull()
  })

  it('removes a profile by id', async () => {
    listed = [profile('01JGLM', { label: 'GLM' })]
    known = []
    await shown()
    fireEvent.click(screen.getByLabelText('Show how GLM is started'))
    fireEvent.click(screen.getByText('Remove'))
    await waitFor(() => expect(removed).toHaveBeenCalledWith('01JGLM'))
  })

  it('says why a removal was refused', async () => {
    listed = [profile('01JGLM', { label: 'GLM' })]
    known = []
    await shown()
    fireEvent.click(screen.getByLabelText('Show how GLM is started'))
    refusal = 'a step in devpit still uses this profile: Review'
    fireEvent.click(screen.getByText('Remove'))
    await waitFor(() => expect(screen.getByText(/still uses this profile/)).toBeTruthy())
  })

  it('hides a token until it is asked for', async () => {
    listed = [
      profile('01JGLM', {
        label: 'GLM',
        env: [{ name: 'ANTHROPIC_AUTH_TOKEN', value: 'sk-secret-value' }],
      }),
    ]
    known = []
    await shown()
    fireEvent.click(screen.getByLabelText('Show how GLM is started'))
    expect(screen.queryByDisplayValue('sk-secret-value')).toBeNull()
    fireEvent.click(screen.getByLabelText('Show ANTHROPIC_AUTH_TOKEN'))
    expect(screen.getByDisplayValue('sk-secret-value')).toBeTruthy()
  })

  it('never puts a value in the row itself', async () => {
    listed = [
      profile('01JGLM', {
        label: 'GLM',
        env: [{ name: 'ANTHROPIC_AUTH_TOKEN', value: 'sk-secret-value' }],
      }),
    ]
    known = []
    await shown()
    expect(screen.queryByText(/sk-secret-value/)).toBeNull()
  })
})

describe('a switch you can switch back', () => {
  it('still shows an agent that was switched off', async () => {
    // Filtering the list the pane itself reads is how an agent turned off
    // becomes an agent nobody can turn back on.
    known = [agent('codex', { label: 'Codex', enabled: false })]
    await shown()
    expect(screen.getByText('Codex')).toBeTruthy()
    expect(screen.getByText('Enabled')).toBeTruthy()
  })

  it('turns it back on', async () => {
    known = [agent('codex', { label: 'Codex', enabled: false })]
    await shown()
    fireEvent.click(screen.getByText('Enabled'))
    await waitFor(() => expect(switched).toHaveBeenCalledWith('codex', true))
  })

  it('keeps it out of the default picker while it is off', async () => {
    known = [agent('codex', { label: 'Codex', enabled: false })]
    await shown()
    const picker = screen.getByRole('radiogroup', { name: 'Default agent' })
    expect(picker.textContent).not.toContain('Codex')
  })
})

describe('finding a row again', () => {
  it('gives every agent a mark of its own', async () => {
    const { MARKED } = await import('./AgentGlyph')
    // Every agent this build can start is one somebody has to pick out of a
    // list of twelve.
    expect(MARKED.length).toBeGreaterThanOrEqual(12)
    expect(new Set(MARKED).size).toBe(MARKED.length)
  })

  it('draws a mark on each row', async () => {
    known = [agent('claude', { label: 'Claude Code' })]
    const { container } = render(<ProviderRows />)
    await waitFor(() => expect(container.querySelector('.agmark')).toBeTruthy())
  })

  it('lends a profile the mark of what it runs', async () => {
    // `GLM` is a Claude Code, and looks like one in the list.
    listed = [profile('01JGLM', { label: 'GLM', base: 'claude' })]
    known = []
    const { container } = render(<ProviderRows />)
    await waitFor(() => expect(container.querySelector('.agmark')).toBeTruthy())
  })

  /* The address stays in the href, to hover over and copy, but the click is
     the app's: nothing in the window answers a new-window request, so
     target="_blank" opened nothing at all. */
  it('links an agent to its own documentation, and opens it through the app', async () => {
    known = [agent('claude', { label: 'Claude Code', homepage: 'https://code.claude.com/docs' })]
    await shown()
    const link = screen.getByLabelText('Claude Code documentation')
    expect(link.getAttribute('href')).toBe('https://code.claude.com/docs')
    expect(link.getAttribute('target')).toBeNull()

    opened.length = 0
    fireEvent.click(link)
    await waitFor(() => expect(opened).toEqual(['https://code.claude.com/docs']))
  })

  it('has no link for a profile, which has no page to send anyone to', async () => {
    listed = [profile('01JGLM', { label: 'GLM' })]
    known = []
    await shown()
    expect(screen.queryByLabelText('GLM documentation')).toBeNull()
  })
})

describe('the shape of the pane', () => {
  it('does not put `not on this machine` under a heading that says Installed', async () => {
    // The heading arguing with its own rows.
    known = [agent('here', { label: 'Here' }), agent('gone', { label: 'Gone', installed: false })]
    await shown()
    expect(screen.getByText('1 detected')).toBeTruthy()
    expect(screen.getByText(/Not on this machine/)).toBeTruthy()
  })

  it('says nothing about a second group when everything is here', async () => {
    known = [agent('here')]
    await shown()
    expect(screen.queryByText(/Not on this machine/)).toBeNull()
  })

  it('lets the progress reporting be switched off', async () => {
    await shown()
    const row = screen.getByRole('switch', { name: /Agent status hooks/ })
    expect(row.getAttribute('aria-checked')).toBe('true')
    fireEvent.click(row)
    await waitFor(() => expect(hooked).toHaveBeenCalledWith(false))
  })

  it('marks each chip in the picker', async () => {
    known = [agent('claude', { label: 'Claude Code' })]
    await shown()
    const picker = screen.getByRole('radiogroup', { name: 'Default agent' })
    expect(picker.querySelectorAll('.agmark').length).toBeGreaterThan(0)
  })

  it('ticks the one that is chosen', async () => {
    choice = { defaultId: 'claude', disabled: [], hooks: true }
    await shown()
    const picker = screen.getByRole('radiogroup', { name: 'Default agent' })
    expect(picker.textContent).toContain('✓')
  })
})
