import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import type { PaneRunning } from '../gen/bindings'
import { SessionRows } from './SessionRows'
import { StopRunning } from './StopRunning'
import type { Tab } from './strip'
import { TabStrip } from './TabStrip'

afterEach(cleanup)

/* jsdom has neither of these, and the strip animates a tab that moved. Both
   are the browser's, not the app's: what is tested here is what the strip
   draws, not how it slides. */
beforeAll(() => {
  window.matchMedia ??= (() => ({ matches: false })) as unknown as typeof window.matchMedia
  Element.prototype.animate ??= (() => ({ finished: Promise.resolve() })) as unknown as typeof Element.prototype.animate
})

/*
 * A terminal with an agent open used to look exactly like an empty one, in
 * the strip and in the sidebar both. Two causes, and each is covered here: the
 * tab never recorded which panes it was showing, so the lookup could not
 * match; and the backend named the executable, which for every JavaScript
 * agent is `node`.
 */

const shell = {
  open: [] as Tab[],
  active: null as Tab | null,
  running: [] as PaneRunning[],
  doing: {} as Record<string, string>,
  focus: vi.fn(),
  close: vi.fn(),
  move: vi.fn(),
  rename: vi.fn(),
  show: vi.fn(),
  openPalette: vi.fn(),
  renaming: null as { id: string; where: string } | null,
  setRenaming: vi.fn(),
}

vi.mock('./useShell', () => ({ useShell: () => shell }))

const claude: PaneRunning = {
  paneId: 'leaf_1',
  command: 'claude',
  busy: true,
  agent: 'claude',
  label: 'Claude Code',
}

const build: PaneRunning = {
  paneId: 'leaf_1',
  command: 'cargo',
  busy: true,
  agent: null,
  label: 'cargo',
}

const idle: PaneRunning = {
  paneId: 'leaf_1',
  command: 'zsh',
  busy: false,
  agent: null,
  label: 'zsh',
}

const term = (over: Partial<Tab> = {}): Tab => ({
  id: 't1',
  kind: 'term',
  title: 'Terminal 1',
  panes: ['leaf_1'],
  ...over,
})

beforeEach(() => {
  shell.open = []
  shell.running = []
  shell.doing = {}
  shell.renaming = null
  shell.active = null
})

describe('a terminal running an agent, in the sidebar', () => {
  it('names the agent instead of saying `terminal`', () => {
    shell.open = [term()]
    shell.running = [claude]
    render(<SessionRows />)
    expect(screen.getByText('Claude Code')).toBeTruthy()
    expect(screen.queryByText('terminal')).toBeNull()
  })

  it('names an ordinary command too', () => {
    shell.open = [term()]
    shell.running = [build]
    render(<SessionRows />)
    expect(screen.getByText('cargo')).toBeTruthy()
  })

  it('says nothing about a pane sitting at its prompt', () => {
    shell.open = [term()]
    shell.running = [idle]
    render(<SessionRows />)
    expect(screen.getByText('terminal')).toBeTruthy()
    expect(screen.queryByText('zsh')).toBeNull()
  })

  /* The lookup that never matched: the running list is keyed by leaf, and a
     tab that has not said which leaves it draws matches nothing. */
  it('shows nothing for a tab that has not opened its panes yet', () => {
    shell.open = [{ id: 't1', kind: 'term', title: 'Terminal 1' }]
    shell.running = [claude]
    render(<SessionRows />)
    expect(screen.queryByText('Claude Code')).toBeNull()
  })
})

describe('a terminal running an agent, in the strip', () => {
  it('takes the agent as its name while the name is one we generated', () => {
    shell.open = [term()]
    shell.running = [claude]
    render(<TabStrip />)
    expect(screen.getByText('Claude Code')).toBeTruthy()
    expect(screen.queryByText('Terminal 1')).toBeNull()
  })

  it('keeps a name the person typed', () => {
    shell.open = [term({ title: 'deploy' })]
    shell.running = [claude]
    render(<TabStrip />)
    expect(screen.getByText('deploy')).toBeTruthy()
  })

  it('leaves the name alone for a command that is not an agent', () => {
    shell.open = [term()]
    shell.running = [build]
    render(<TabStrip />)
    expect(screen.getByText('Terminal 1')).toBeTruthy()
  })

  /* Orca's plus opens the field rather than a bare terminal, because most of
     the time what people came to start is an agent. */
  it('opens the field from the plus', () => {
    shell.open = []
    shell.openPalette.mockClear()
    render(<TabStrip />)
    fireEvent.click(screen.getByLabelText('Open something new'))
    expect(shell.openPalette).toHaveBeenCalled()
  })
})

describe('closing a terminal that is still working', () => {
  it('words an agent as an agent', () => {
    render(
      <StopRunning
        closing={{ id: 't1', tab: 'Terminal 1', stops: { kind: 'agent', label: 'Claude Code' } }}
        onCancel={vi.fn()}
        onConfirm={vi.fn()}
      />,
    )
    expect(screen.getByText('Stop this agent?')).toBeTruthy()
    expect(screen.getByText('Stop Agent')).toBeTruthy()
  })

  it('words a command as a command', () => {
    render(
      <StopRunning
        closing={{ id: 't1', tab: 'Terminal 1', stops: { kind: 'command', label: 'cargo' } }}
        onCancel={vi.fn()}
        onConfirm={vi.fn()}
      />,
    )
    expect(screen.getByText('Stop running command?')).toBeTruthy()
    expect(screen.getByText('Stop and Close')).toBeTruthy()
  })

  /* The tick is a decision. Confirming without it must not turn the prompt
     off, or pressing Enter twice would silently disable it for good. */
  it('only opts out when the box is ticked', () => {
    const onConfirm = vi.fn()
    render(
      <StopRunning
        closing={{ id: 't1', tab: 'Terminal 1', stops: { kind: 'agent', label: 'Claude Code' } }}
        onCancel={vi.fn()}
        onConfirm={onConfirm}
      />,
    )
    fireEvent.click(screen.getByText('Stop Agent'))
    expect(onConfirm).toHaveBeenCalledWith(false)

    fireEvent.click(screen.getByRole('checkbox'))
    fireEvent.click(screen.getByText('Stop Agent'))
    expect(onConfirm).toHaveBeenLastCalledWith(true)
  })

  it('goes back without closing anything', () => {
    const onCancel = vi.fn()
    render(
      <StopRunning
        closing={{ id: 't1', tab: 'Terminal 1', stops: { kind: 'command', label: 'cargo' } }}
        onCancel={onCancel}
        onConfirm={vi.fn()}
      />,
    )
    fireEvent.click(screen.getByText('Cancel'))
    expect(onCancel).toHaveBeenCalled()
  })
})

/*
 * What the agent says about itself, which is the half the process table
 * cannot see: an agent blocked on the network and an agent blocked on you
 * look identical from outside. It arrives on the agent's own hooks, and only
 * for agents this app started.
 */
describe('what the agent says it is doing', () => {
  it('says so in the sidebar when it is waiting on a person', () => {
    shell.open = [term()]
    shell.running = [claude]
    shell.doing = { leaf_1: 'waiting' }
    render(<SessionRows />)
    expect(screen.getByText('Claude Code · waiting on you')).toBeTruthy()
  })

  it('says only the name while it is working', () => {
    shell.open = [term()]
    shell.running = [claude]
    shell.doing = { leaf_1: 'working' }
    render(<SessionRows />)
    expect(screen.getByText('Claude Code')).toBeTruthy()
  })

  /* An agent started outside this app was never asked to report, so nothing
     is known about it. A missing answer is missing, not idle. */
  it('says nothing extra about an agent that never reported', () => {
    shell.open = [term()]
    shell.running = [claude]
    render(<SessionRows />)
    expect(screen.getByText('Claude Code')).toBeTruthy()
  })

  it('does not take another pane\'s report for this one', () => {
    shell.open = [term()]
    shell.running = [claude]
    shell.doing = { leaf_somewhere_else: 'waiting' }
    render(<SessionRows />)
    expect(screen.getByText('Claude Code')).toBeTruthy()
  })
})

/*
 * The strip with more tabs than fit.
 *
 * It was `flex: none`, so it never gave up width: with eight files open it
 * pushed the branch chip and the notice bell off the right edge of the window.
 */
describe('a strip with more tabs than room', () => {
  const many = (): Tab[] =>
    Array.from({ length: 9 }, (_, at) =>
      term({ id: `t${at}`, title: `file-with-a-long-name-${at}.tsx`, panes: [`leaf_${at}`] }),
    )

  it('keeps the tabs in a rail of their own', () => {
    shell.open = many()
    const { container } = render(<TabStrip />)
    expect(container.querySelector('.tabs__rail')).toBeTruthy()
  })

  it('leaves the plus outside the rail, so it never scrolls away', () => {
    shell.open = many()
    const { container } = render(<TabStrip />)
    const plus = container.querySelector('.tab--new')!
    expect(plus.closest('.tabs__rail')).toBeNull()
    expect(plus.closest('.tabs')).toBeTruthy()
  })

  it('does not count the plus as a tab when reordering', () => {
    // The drag lands on an index read from the rail's children. With the plus
    // inside, dropping past the last tab could aim at the button.
    shell.open = many()
    const { container } = render(<TabStrip />)
    const rail = container.querySelector('.tabs__rail')!
    expect(rail.children).toHaveLength(9)
  })

  it('gives the label an element of its own, and the whole name to trim', () => {
    // It used to be cut at 22 characters in JavaScript, which cut it whether
    // or not there was room. The element is what lets CSS trim it only when
    // the tab is actually too narrow.
    shell.open = [term({ title: 'a-very-long-filename-indeed.tsx' })]
    const { container } = render(<TabStrip />)
    expect(container.querySelector('.tab__n')?.textContent).toBe('a-very-long-filename-indeed.tsx')
  })

  it('puts the whole name in the tooltip too', () => {
    shell.open = [term({ title: 'a-very-long-filename-indeed.tsx' })]
    const { container } = render(<TabStrip />)
    expect(container.querySelector('.tab')?.getAttribute('title')).toBe(
      'a-very-long-filename-indeed.tsx',
    )
  })

  it('brings the active tab into view rather than leaving it scrolled past', () => {
    // Picked from the palette, the tab that becomes active can be one the
    // strip has scrolled past. Selecting something nobody can see is the
    // failure the scrolling introduced.
    const brought: string[] = []
    Element.prototype.scrollIntoView = function (this: Element) {
      brought.push(this.getAttribute('data-tab') ?? '')
    }
    shell.open = many()
    shell.active = shell.open[7]!
    render(<TabStrip />)
    expect(brought).toContain('t7')
  })
})
