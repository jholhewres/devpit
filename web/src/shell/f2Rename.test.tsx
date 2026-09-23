import { cleanup, fireEvent, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ContextMenu } from './ContextMenu'

/* The menus say F2 renames; one F2 is one rename, on a file row and on a
   session row alike — heard once, by the window's one listener. */
const run = vi.fn()
const setRenaming = vi.fn()
vi.mock('./useFileActions', () => ({
  useFileActions: () => ({ run, naming: null, renaming: null, deleting: null, close: vi.fn(), create: vi.fn(), rename: vi.fn(), destroy: vi.fn() }),
}))
vi.mock('./FileDialogs', () => ({ FileDialogs: () => null }))
const shell = { focus: vi.fn(), close: vi.fn(), sweep: vi.fn(), join: vi.fn(), open: [], setRenaming }
vi.mock('./useShell', () => ({ useShell: () => shell }))

afterEach(() => {
  cleanup()
  run.mockClear()
  setRenaming.mockClear()
})

describe('F2 on a row whose menu offers it', () => {
  it('renames a file once', () => {
    const { getByText } = render(
      <>
        <ContextMenu />
        <div data-ctx="file" data-path="src/a.ts" tabIndex={0}>
          a.ts
        </div>
      </>,
    )
    fireEvent.keyDown(getByText('a.ts'), { key: 'F2' })
    expect(run).toHaveBeenCalledTimes(1)
    expect(run).toHaveBeenCalledWith('rename', 'src/a.ts')
  })

  it('renames a session once', () => {
    const { getByText } = render(
      <>
        <ContextMenu />
        <div data-ctx="session" data-id="tab_1" tabIndex={0}>
          a chat
        </div>
      </>,
    )
    fireEvent.keyDown(getByText('a chat'), { key: 'F2' })
    expect(setRenaming).toHaveBeenCalledTimes(1)
  })
})
