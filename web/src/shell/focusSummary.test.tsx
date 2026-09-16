import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Notice } from '../gen/bindings'
import { FocusSummary } from './FocusSummary'

let pages: { notices: Notice[]; more: boolean }[] = []
const asked: (string | null)[] = []

vi.mock('./live', () => ({
  ask: async (call: () => Promise<{ data?: unknown }>) => ({
    data: (await call()) ?? null,
    error: null,
    loading: false,
  }),
  commands: {
    focusWaiting: async (_project: string, _since: number, after: string | null) => {
      asked.push(after)
      return pages.shift() ?? { notices: [], more: false }
    },
  },
}))

const notice = (over: Partial<Notice>): Notice => ({
  id: 'ntc_1',
  projectId: 'prj_far',
  kind: 'run',
  title: 'tests finished',
  detail: null,
  cardId: 'card_1',
  createdAt: 1000,
  readAt: null,
  ...over,
})

afterEach(() => {
  cleanup()
  pages = []
  asked.length = 0
})

describe('what a focus says it held, on the way out', () => {
  const ended = { projectId: 'prj_here', since: 1000 }

  it('says so plainly when nothing arrived', async () => {
    render(<FocusSummary ended={ended} onOpenCard={vi.fn()} onClose={vi.fn()} />)
    await waitFor(() =>
      expect(screen.getByText('Nothing arrived while you were in it')).toBeTruthy(),
    )
  })

  /* A focus that lasted a day is exactly the case the bell's own page cannot
     answer, so the summary walks forward until the pages run out. */
  it('walks every page, asking from the last one it saw', async () => {
    pages = [
      { notices: [notice({ id: 'a' }), notice({ id: 'b' })], more: true },
      { notices: [notice({ id: 'c' })], more: false },
    ]
    render(<FocusSummary ended={ended} onOpenCard={vi.fn()} onClose={vi.fn()} />)

    await waitFor(() => expect(screen.getByText(/3 arrived/)).toBeTruthy())
    expect(asked).toEqual([null, 'b'])
  })

  it('groups by the project it came from, and what waits on you comes first', async () => {
    pages = [
      {
        notices: [
          notice({ id: 'run', kind: 'run', title: 'a run ended' }),
          notice({ id: 'agent', kind: 'agent', title: 'an agent is waiting on you' }),
        ],
        more: false,
      },
    ]
    render(<FocusSummary ended={ended} onOpenCard={vi.fn()} onClose={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('prj_far')).toBeTruthy())
    // The rows themselves, not the Done beside the title.
    const rows = [...document.querySelectorAll('.fsum__one')].map((node) => node.textContent)
    expect(rows[0]).toContain('an agent is waiting on you')
    expect(rows[1]).toContain('a run ended')
  })

  it('acts in place: a row opens its card', async () => {
    const onOpenCard = vi.fn()
    pages = [{ notices: [notice({ id: 'a', cardId: 'card_9' })], more: false }]
    render(<FocusSummary ended={ended} onOpenCard={onOpenCard} onClose={vi.fn()} />)

    fireEvent.click(await screen.findByText('tests finished'))
    expect(onOpenCard).toHaveBeenCalledWith('card_9')
  })
})
