import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { DevflowProfile } from '../gen/bindings'

const choices = [
  { id: 'claudin', label: 'Claudin' },
  { id: 'glm', label: 'GLM' },
]
let now: DevflowProfile = { projectId: 'prj_api', named: null, runsAs: 'Claudin', from: 'orchestrator', choices }
const set = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null }),
  commands: {
    devflowProfile: async () => now,
    devflowProfileSet: async (projectId: string, id: string | null) => {
      set(projectId, id)
      now = { ...now, named: id, runsAs: id === 'glm' ? 'GLM' : 'Claudin', from: id ? 'project' : 'orchestrator' }
      return now
    },
  },
}))

const { ReviewersAccount } = await import('./ReviewersAccount')

describe('the account the reviewers run as', () => {
  it('says which and why, and the project can name another', async () => {
    render(<ReviewersAccount projectId="prj_api" />)
    expect(await screen.findByText(/Claudin: the account of the orchestrators linked to this project/)).toBeTruthy()
    fireEvent.change(screen.getByLabelText('Account the reviewers run as'), { target: { value: 'glm' } })
    await waitFor(() => expect(set).toHaveBeenCalledWith('prj_api', 'glm'))
    expect(await screen.findByText(/GLM: named by this project/)).toBeTruthy()
  })
})
