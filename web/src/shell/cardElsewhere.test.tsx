import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { CardElsewhere } from './CardElsewhere'

let repos: unknown[] = []
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null }),
  commands: { cardElsewhere: () => repos },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ setProject: vi.fn() }) }))
vi.mock('./RightPanel', () => ({ SHOW_PANEL: 'devpit:show-panel' }))

afterEach(() => {
  cleanup()
  repos = []
})

describe('what a card changed in other repositories', () => {
  it('says nothing when its sessions stayed in its checkout', () => {
    const { container } = render(<CardElsewhere cardId="card_1" />)
    expect(container.textContent).toBe('')
  })

  it('names the other repository, how many files and how many are not committed', async () => {
    repos = [
      {
        root: '/w/devpit-app',
        name: 'devpit-app',
        projectId: 'prj_app',
        files: [
          { path: 'crates/api/client_ip.rs', uncommitted: true },
          { path: 'crates/db/reports.rs', uncommitted: false },
        ],
      },
    ]
    render(<CardElsewhere cardId="card_1" />)
    expect(await screen.findByText('Also changed 2 files in devpit-app')).toBeTruthy()
    expect(screen.getByText('1 uncommitted')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Open devpit-app' })).toBeTruthy()
  })
})
