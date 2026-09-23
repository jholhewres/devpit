import { cleanup, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { Panes } from './Panes'
import { PANES } from './paneList'
import type { Tab } from './strip'

/* Each leaf pane is tested on its own; here only whether it mounts matters. */
vi.mock('./BoardPane', () => ({ BoardPane: () => <div>board</div> }))
vi.mock('./BrowserPane', () => ({ BrowserPane: () => <div>browser</div> }))
vi.mock('./ChatPane', () => ({ ChatPane: () => <div>chat</div> }))
vi.mock('./SkillsPane', () => ({ SkillsPane: () => <div>skills</div> }))
vi.mock('./DiffPane', () => ({ DiffPane: () => <div>diff</div> }))
vi.mock('./FilePane', () => ({ FilePane: () => <div>file</div> }))
vi.mock('./FilesPane', () => ({ FilesPane: () => <div>files</div> }))
vi.mock('./McpPane', () => ({ McpPane: () => <div>mcps</div> }))
vi.mock('./PluginsPane', () => ({ PluginsPane: () => <div>plugins</div> }))
vi.mock('../plugins/excalidraw/Drawing', () => ({ Drawing: () => <div>drawing</div> }))
vi.mock('../plugins/excalidraw/DrawingList', () => ({ DrawingList: () => <div>drawings</div> }))
vi.mock('./TerminalPane', () => ({ TerminalPane: () => <div>term</div> }))
vi.mock('./WorkspacePane', () => ({ WorkspacePane: () => <div>workspace</div> }))

/* One open tab per kind you can have several of, so every many-instance
   mount has a tab to render from. A single-instance kind mounts once it is
   the one in front, and stays. */
const open: Tab[] = [
  { id: 'browser_1', kind: 'browser' },
  { id: 'chat_1', kind: 'chat' },
  { id: 'diff_1', kind: 'diff' },
  { id: 'drawing_1', kind: 'drawing' },
  { id: 'file_1', kind: 'file' },
  { id: 'note_1', kind: 'note' },
  { id: 'term_1', kind: 'term' },
]
/* `projects` is empty rather than absent: the Manager reads the list to fill
   its filter, and a shell without one is a shell no window ever has. */
const shell: { open: Tab[]; active: Tab | null; [key: string]: unknown } = {
  open,
  active: null,
  show: vi.fn(),
  close: vi.fn(),
  project: null,
  projects: [],
  setProject: vi.fn(),
  openCard: vi.fn(),
}
vi.mock('./useShell', () => ({ useShell: () => shell }))

afterEach(cleanup)

describe('the pane mount table', () => {
  it('opens every pane name to an element carrying it', () => {
    const { rerender } = render(<Panes />)
    for (const { name } of PANES) {
      if (!document.querySelector(`[data-pane="${name}"]`)) {
        shell.active = { id: name, kind: name }
        rerender(<Panes />)
      }
      expect(document.querySelector(`[data-pane="${name}"]`), `missing data-pane for ${name}`).not.toBeNull()
    }
    shell.active = null
  })

  it('mounts a single-instance pane only once it has been opened, then keeps it', () => {
    shell.active = null
    const { rerender } = render(<Panes />)
    expect(document.querySelector('[data-pane="skills"]')).toBeNull()
    shell.active = { id: 'skills', kind: 'skills' }
    rerender(<Panes />)
    expect(document.querySelector('[data-pane="skills"]')).not.toBeNull()
    shell.active = null
    rerender(<Panes />)
    expect(document.querySelector('[data-pane="skills"]')).not.toBeNull()
  })

  /* The board hears sessions and progress only as events, so it is there
     from the start even if nobody opened it. */
  it('mounts the board at start', () => {
    shell.active = null
    render(<Panes />)
    expect(document.querySelector('[data-pane="board"]')).not.toBeNull()
  })
})
