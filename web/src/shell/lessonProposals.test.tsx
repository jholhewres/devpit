import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LessonProposal, LessonProposals as Proposals } from '../gen/bindings'
import { LessonProposals } from './LessonProposals'

vi.mock('./window', () => ({
  onCarried: () => () => undefined,
}))

const proposed = (source: string, text: string): LessonProposal => ({
  projectId: 'prj_api',
  project: 'asc-api',
  day: '2026-11-13',
  source,
  text,
})

let waiting: LessonProposal[] = []
const decided = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (why) {
      return { data: null, error: String(why), loading: false }
    }
  },
  commands: {
    lessonsProposed: async (): Promise<Proposals> => ({ proposals: waiting }),
    lessonDecide: async (projectId: string, source: string, text: string, kept: string | null): Promise<Proposals> => {
      decided(projectId, source, text, kept)
      waiting = waiting.filter((one) => one.text !== text)
      return { proposals: waiting }
    },
  },
}))

afterEach(() => {
  cleanup()
  waiting = []
  decided.mockReset()
})

describe('LessonProposals', () => {
  it('draws nothing while no lesson waits', async () => {
    render(<LessonProposals />)
    await waitFor(() => expect(screen.queryByRole('region')).toBeNull())
  })

  it('approves a lesson as proposed, and it goes', async () => {
    waiting = [proposed('#331', 'Wait 3 s for the queue test.')]
    render(<LessonProposals />)
    fireEvent.click(await screen.findByRole('button', { name: 'Approve' }))
    await waitFor(() => expect(decided).toHaveBeenCalledWith('prj_api', '#331', 'Wait 3 s for the queue test.', 'Wait 3 s for the queue test.'))
    await waitFor(() => expect(screen.queryByText('Wait 3 s for the queue test.')).toBeNull())
  })

  it('approves a lesson as edited', async () => {
    waiting = [proposed('#331', 'Wait 3 s for the queue test.')]
    render(<LessonProposals />)
    fireEvent.click(await screen.findByRole('button', { name: 'Edit' }))
    fireEvent.change(screen.getByRole('textbox', { name: 'Lesson' }), { target: { value: 'Wait 3 s under load.' } })
    fireEvent.click(screen.getByRole('button', { name: 'Approve' }))
    await waitFor(() => expect(decided).toHaveBeenCalledWith('prj_api', '#331', 'Wait 3 s for the queue test.', 'Wait 3 s under load.'))
  })

  it('discards a lesson, keeping nothing', async () => {
    waiting = [proposed('#331', 'Run make fmt.')]
    render(<LessonProposals />)
    fireEvent.click(await screen.findByRole('button', { name: 'Discard' }))
    await waitFor(() => expect(decided).toHaveBeenCalledWith('prj_api', '#331', 'Run make fmt.', null))
  })

  it("shows a card only the lessons it proposed", async () => {
    waiting = [proposed('#331', 'Mine.'), proposed('#12', 'Not mine.')]
    render(<LessonProposals projectId="prj_api" source="#331" />)
    expect(await screen.findByText('Mine.')).toBeTruthy()
    expect(screen.queryByText('Not mine.')).toBeNull()
  })
})
