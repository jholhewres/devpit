import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ArtifactsView } from './ArtifactsView'

const removed = vi.fn()
const shown = vi.fn()
let items = [
  { name: 'notes.md', bytes: 120, modified: null },
  { name: 'specs/api.md', bytes: 2048, modified: null },
]

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    artifactsList: () => ({ folder: '/home/me/.devpit/projects/app/artifacts', items }),
    artifactRemove: (projectId: string, name: string) => (removed(projectId, name), (items = items.filter((one) => one.name !== name)), null),
    pathReveal: () => null,
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'prj_app' }, show: shown }) }))

afterEach(() => {
  cleanup()
  removed.mockReset()
  shown.mockReset()
})

describe("a project's artifacts", () => {
  it('lists each by name and opens one from its whole path', async () => {
    render(<ArtifactsView shown />)
    fireEvent.click(await screen.findByText('specs/api.md'))
    expect(shown).toHaveBeenCalledWith('file', expect.objectContaining({ path: '/home/me/.devpit/projects/app/artifacts/specs/api.md' }))
  })

  it('removes one only after asking, in place', async () => {
    render(<ArtifactsView shown />)
    fireEvent.click(await screen.findByLabelText('Remove notes.md'))
    expect(removed).not.toHaveBeenCalled()
    fireEvent.click(screen.getByText('Remove?'))
    await waitFor(() => expect(removed).toHaveBeenCalledWith('prj_app', 'notes.md'))
    await waitFor(() => expect(screen.queryByText('notes.md')).toBeNull())
  })
})
