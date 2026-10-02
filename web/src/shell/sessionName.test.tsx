import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { SessionChanges } from '../gen/bindings'
import { branchWords } from './SessionChangesPanel'
import { SessionName } from './SessionName'

const renamed = vi.fn((..._: unknown[]): null => null)
let refusal: string | null = null
vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve(refusal ? { data: null, error: refusal } : { data: call(), error: null }),
  commands: { orchestratorRename: (...args: unknown[]) => renamed(...args) },
}))

afterEach(() => {
  cleanup()
  renamed.mockClear()
  refusal = null
})

describe("a session's name in its window", () => {
  it('renames through the orchestrator with Enter, and says the new name', async () => {
    const onRenamed = vi.fn()
    render(<SessionName profileId="prof" name="devpit-02" onRenamed={onRenamed} />)
    fireEvent.click(screen.getByRole('button', { name: 'devpit-02' }))
    const field = screen.getByLabelText('Session name')
    fireEvent.change(field, { target: { value: 'invoice-fix' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    await waitFor(() => expect(onRenamed).toHaveBeenCalledWith('invoice-fix'))
    expect(renamed).toHaveBeenCalledWith('prof', 'devpit-02', 'invoice-fix')
  })

  it('keeps the field and says why when the rename is refused', async () => {
    refusal = 'a session is already called api'
    render(<SessionName profileId="prof" name="devpit-02" onRenamed={vi.fn()} />)
    fireEvent.click(screen.getByRole('button', { name: 'devpit-02' }))
    const field = screen.getByLabelText('Session name')
    fireEvent.change(field, { target: { value: 'api' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(await screen.findByText('a session is already called api')).toBeTruthy()
  })

  it('is only words where there is no orchestrator to ask', () => {
    render(<SessionName profileId={null} name="devpit-02" onRenamed={vi.fn()} />)
    expect(screen.queryByRole('button')).toBeNull()
  })

  it('says the branch, its drift and the checkout', () => {
    const changes = { branch: 'devpit/invoice', ahead: 2, behind: 0, worktree: true, folder: 'invoice-01' } as SessionChanges
    expect(branchWords(changes)).toBe('devpit/invoice · ↑2 · invoice-01')
    expect(branchWords({ ...changes, ahead: 0, worktree: false })).toBe('devpit/invoice · project folder')
  })
})
