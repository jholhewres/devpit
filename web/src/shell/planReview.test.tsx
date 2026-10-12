import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Card, DevflowPlan, PlannedCard } from '../gen/bindings'
import { CardCriteria } from './CardCriteria'
import { Tile } from './Lane'
import { PlanReview } from './PlanReview'

const kept = vi.fn()
const created = vi.fn()
const dropped = vi.fn()
/* What the backend did, in the order it finished. */
const landed: string[] = []
/* How long a keep takes on its way back; 0 lands it at once. */
let slowKeep = 0
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    devflowPlanKeep: async (_project: string, epicId: string, cards: PlannedCard[]) => {
      kept(cards)
      if (slowKeep > 0) await new Promise((done) => setTimeout(done, slowKeep))
      landed.push('kept')
      return { epicId, cards, edited: true, plannedAt: 1 }
    },
    devflowPlanCreate: async (_project: string, epicId: string) => {
      landed.push('created')
      created(epicId)
      return { cardIds: ['card_1', 'card_2', 'card_3'] }
    },
    devflowPlanDrop: async (_project: string, epicId: string) => {
      dropped(epicId)
      return null
    },
  },
}))

afterEach(() => {
  cleanup()
  kept.mockReset()
  created.mockReset()
  dropped.mockReset()
  landed.length = 0
  slowKeep = 0
})

const planned = (key: string, title: string, dependsOn: string[] = []): PlannedCard => ({
  key,
  title,
  build: '',
  criteria: [`${title} works`],
  files: [],
  verify: 'make test',
  dependsOn,
})

const plan: DevflowPlan = {
  epicId: 'card_epic',
  cards: [planned('s1', 'Index bodies'), planned('s2', 'Search box', ['s1']), planned('s3', 'Highlight', ['s2'])],
  edited: false,
  plannedAt: 1,
}

describe('a plan waiting on its epic', () => {
  it('shows each card with what blocks it, and creates nothing until the click', async () => {
    const changed = vi.fn()
    render(<PlanReview projectId="prj" plan={plan} onChanged={changed} />)
    expect(screen.getByText('Plan · 3 cards to create')).toBeTruthy()
    expect(screen.getByText('Blocked by Index bodies')).toBeTruthy()
    expect(created).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Create 3 cards' }))
    await waitFor(() => expect(created).toHaveBeenCalledWith('card_epic'))
    await waitFor(() => expect(changed).toHaveBeenCalled())
  })

  it('keeps a cut, and the card after it no longer waits on the one cut', async () => {
    render(<PlanReview projectId="prj" plan={plan} onChanged={vi.fn()} />)
    fireEvent.click(screen.getByRole('button', { name: 'Cut card 2' }))
    await waitFor(() => expect(kept).toHaveBeenCalled())
    const cards = kept.mock.calls[0]![0] as PlannedCard[]
    expect(cards.map((card) => card.key)).toEqual(['s1', 's3'])
    expect(cards[1]!.dependsOn).toEqual([])
    expect(screen.getByRole('button', { name: 'Create 2 cards' })).toBeTruthy()
  })

  it('keeps an edit when the field is left, and nothing when it was not changed', async () => {
    render(<PlanReview projectId="prj" plan={plan} onChanged={vi.fn()} />)
    const title = screen.getByLabelText('Title of card 1')
    fireEvent.blur(title)
    expect(kept).not.toHaveBeenCalled()
    fireEvent.change(title, { target: { value: 'Index card bodies' } })
    fireEvent.blur(title)
    await waitFor(() => expect(kept).toHaveBeenCalledTimes(1))
    expect((kept.mock.calls[0]![0] as PlannedCard[])[0]!.title).toBe('Index card bodies')
  })

  it('creates the cards only after the edit the click left behind has landed', async () => {
    slowKeep = 30
    render(<PlanReview projectId="prj" plan={plan} onChanged={vi.fn()} />)
    const title = screen.getByLabelText('Title of card 3')
    fireEvent.change(title, { target: { value: 'Highlight matches' } })
    fireEvent.blur(title)
    fireEvent.click(screen.getByRole('button', { name: 'Create 3 cards' }))
    await waitFor(() => expect(created).toHaveBeenCalled())
    expect(landed).toEqual(['kept', 'created'])
  })
})

describe('acceptance criteria', () => {
  it('tick the one clicked', () => {
    const ticked = vi.fn()
    render(
      <CardCriteria
        criteria={[
          { text: 'a word finds its card', done: true },
          { text: 'an empty query finds nothing', done: false },
        ]}
        onTick={ticked}
      />,
    )
    expect(screen.getByText('Acceptance criteria · 1 of 2')).toBeTruthy()
    fireEvent.click(screen.getByLabelText('an empty query finds nothing'))
    expect(ticked).toHaveBeenCalledWith(1, true)
  })
})

describe('a tile on the board', () => {
  const tile = (over: Partial<Card>): Card => ({
    id: 'card_1',
    columnId: 'col',
    title: '[devflow] Search box',
    body: '',
    position: 0,
    worktreePath: null,
    dueAt: null,
    dueTime: false,
    costUsd: 0,
    comments: 0,
    pinned: 0,
    runs: [],
    activity: null,
    ...over,
  })

  it('says what still blocks it, and not what is done', () => {
    render(
      <Tile
        card={tile({
          blockedBy: [
            { id: 'a', title: '[devflow] Index bodies', done: false },
            { id: 'b', title: '[devflow] Schema', done: true },
          ],
        })}
      />,
    )
    expect(screen.getByText('Blocked by [devflow] Index bodies')).toBeTruthy()
  })

  it("shows an epic's progress, its criteria and a plan waiting", () => {
    render(
      <Tile
        card={tile({
          progress: { done: 1, total: 3 },
          criteria: [{ text: 'one', done: true }, { text: 'two', done: false }],
          planWaiting: 2,
        })}
      />,
    )
    expect(screen.getByTitle('1 of 3 cards done').textContent).toBe('1/3')
    expect(screen.getByTitle('1 of 2 criteria met').textContent).toBe('1/2')
    expect(screen.getByText('Plan · 2')).toBeTruthy()
  })
})
