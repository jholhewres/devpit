import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { COMMUNITY_INVITE, CommunityFoot, CommunityMenuItem } from './Community'

afterEach(cleanup)

const opened: string[] = []

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    urlOpen: (url: string) => (opened.push(url), { path: url }),
  },
}))

describe('the community link', () => {
  it('opens the invite from the account menu and closes the menu', async () => {
    opened.length = 0
    const closed = vi.fn()
    render(<CommunityMenuItem onPick={closed} />)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Community' }))
    await waitFor(() => expect(opened).toEqual([COMMUNITY_INVITE]))
    expect(closed).toHaveBeenCalledOnce()
  })

  it('opens the invite from the foot of settings', async () => {
    opened.length = 0
    render(<CommunityFoot />)
    fireEvent.click(screen.getByRole('button', { name: 'Community' }))
    await waitFor(() => expect(opened).toEqual([COMMUNITY_INVITE]))
  })
})
