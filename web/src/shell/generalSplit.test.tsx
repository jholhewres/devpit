import { act, cleanup, render, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { GeneralPanes } from './GeneralSettings'
import { PrefsSide } from './PrefsSide'
import { landing, ROWS } from './prefsRows'
import type { PrefsPane } from './shape'
import { useOverlays } from './useOverlays'

/* What each read answers: enough for every row to draw, including the ones
   a platform may leave out (the tray, the display). */
const { answers } = vi.hoisted(() => ({
  answers: {
    islandDrawn: { drawn: 'above' },
    pauseRead: { on: false, until: null },
    sessionRelayRead: { nextCard: false, handoffAtPercent: null },
    atLoginRead: false,
    trayRead: { shown: true, keep: true },
    displayRead: { offered: true, running: 'wayland', chosen: null, why: 'default' },
    appsList: [],
    appsKnown: [],
    transcribeRead: { engine: 'local', model: '', language: '', url: '', keySet: false, local: null },
    remoteRead: {
      enabled: true,
      /* Through the relay, the way that also offers remote MCP. */
      via: 'relay',
      relay: { signedIn: true, online: true, problem: null },
      tailscale: { installed: true, running: true, name: 'box.tail1.ts.net', https: true, login: 'me@example.com', ip: null },
      address: null,
      problem: null,
      devices: [],
      mcp: false,
      asking: [],
    },
    calendarRead: { link: null, titles: false, signedIn: true },
    appInfo: { version: '0.1.0', platform: 'linux', statePath: '/x' },
  } as Record<string, unknown>,
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('./window', () => ({ onCarried: () => () => undefined, onEvent: () => () => undefined }))
vi.mock('./useShell', () => ({ useShell: () => ({ closePrefs: vi.fn() }) }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: new Proxy({}, { get: (_, name: string) => async () => answers[name] ?? null }),
}))

afterEach(cleanup)

/* Every row General had before it was split, and the pane that holds it now. */
const MOVED: readonly (readonly [string, string])[] = [
  ['Local by default', 'General'],
  ['Island', 'Notifications'],
  ['Pause', 'Notifications'],
  ['Reminders', 'Notifications'],
  ['Focus mode', 'Notifications'],
  ['Open the next card on its own', 'Sessions'],
  ['Hand off a full session', 'Sessions'],
  ['Open at login', 'General'],
  ['Keep devpit in the tray when the window closes', 'General'],
  ['Shortcut', 'General'],
  ['Display', 'General'],
  ['Ask before stopping a terminal', 'Sessions'],
  ['Open in', 'General'],
  ['Voice messages', 'Voice'],
  ['Remote', 'Remote'],
  ['Allow remote MCP (read-only)', 'Remote'],
  ['Calendar', 'Remote'],
  ['Automatic updates', 'General'],
  ['Updates', 'General'],
  ['Error reports', 'General'],
]

const titled = (root: ParentNode, title: string): HTMLElement[] =>
  Array.from(root.querySelectorAll<HTMLElement>('.pref__t')).filter((node) => {
    const text = node.textContent ?? ''
    return text === title || text.startsWith(`${title} `) || text.startsWith(`${title}·`)
  })

const drawn = async (pane: PrefsPane = 'general'): Promise<HTMLElement> => {
  const { container } = render(<GeneralPanes pane={pane} on={() => false} set={vi.fn()} />)
  await waitFor(() => expect(titled(container, 'Calendar')).toHaveLength(1))
  await waitFor(() => expect(titled(container, 'Voice messages')).toHaveLength(1))
  return container
}

describe('General, split into panes', () => {
  it('keeps every row it had, each once, in the pane it moved to', async () => {
    const root = await drawn()
    for (const [title, pane] of MOVED) {
      const rows = titled(root, title)
      expect(rows, title).toHaveLength(1)
      expect(rows[0]!.closest('section.prefs__in')?.querySelector('h1')?.textContent, title).toBe(pane)
    }
  })

  it('shows only the pane asked for', async () => {
    const root = await drawn('notifications')
    const shown = root.querySelectorAll('section.prefs__in:not([hidden])')
    expect(shown).toHaveLength(1)
    expect(shown[0]!.querySelector('h1')?.textContent).toBe('Notifications')
    expect(titled(shown[0]!, 'Island')).toHaveLength(1)
    expect(titled(shown[0]!, 'Remote')).toHaveLength(0)
  })

  it('draws each row the index names in the pane the index names', async () => {
    for (const [pane, titles] of Object.entries(ROWS)) {
      const root = await drawn(pane as PrefsPane)
      const shown = root.querySelector('section.prefs__in:not([hidden])')!
      for (const title of titles) expect(titled(shown, title), `${pane}: ${title}`).toHaveLength(1)
      cleanup()
    }
  })
})

describe('an address from before the split', () => {
  it('lands on the pane its row moved to', () => {
    expect(landing('general', 'Remote')).toBe('remote')
    expect(landing('general', 'voice messages')).toBe('voice')
    expect(landing('general', 'Island')).toBe('notifications')
    expect(landing('general', 'Ask before stopping a terminal')).toBe('sessions')
    expect(landing('general', 'Calendar')).toBe('remote')
    /* What stayed, stays; and a pane named without a row is that pane. */
    expect(landing('general', 'Shortcut')).toBe('general')
    expect(landing('general')).toBe('general')
    expect(landing('appearance', 'Island')).toBe('appearance')
  })

  it('opens Settings on the new pane and keeps the row to land on', () => {
    const { result } = renderHook(() => useOverlays())
    act(() => result.current.openPrefs('general', 'Voice messages'))
    expect(result.current.prefs).toBe('voice')
    expect(result.current.prefsRow).toBe('Voice messages')
    /* Picking a pane by hand forgets the row. */
    act(() => result.current.openPrefs('channels'))
    expect(result.current.prefs).toBe('channels')
    expect(result.current.prefsRow).toBeNull()
  })

  it('scrolls to the row once it is drawn', async () => {
    const scrolled = vi.fn()
    Element.prototype.scrollIntoView = scrolled
    const { rerender, container } = render(
      <>
        <PrefsSide pane="voice" row="Voice messages" onBack={vi.fn()} onSelect={vi.fn()} />
        <section className="prefs__in" />
      </>,
    )
    /* The pane reads its state first, so the row comes a little later. */
    rerender(
      <>
        <PrefsSide pane="voice" row="Voice messages" onBack={vi.fn()} onSelect={vi.fn()} />
        <section className="prefs__in">
          <div className="pref"><span className="pref__t">Voice messages</span></div>
        </section>
      </>,
    )
    await waitFor(() => expect(scrolled).toHaveBeenCalledOnce())
    expect(scrolled.mock.contexts[0]).toBe(container.querySelector('.pref'))
  })
})
