import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { DeleteRefusal } from '../gen/bindings'
import { CardEnding, CardHeader, deleteBody } from './CardHeader'
import { stylesheet } from './stylesheet'

afterEach(cleanup)

/** Every rule that places the header's buttons, as written. */
const buttonRules = (css: string): string[] =>
  css
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .split('}')
    .filter((rule) => /(^|,|\s)\.(sq26|cardp__acts)\b[^{]*\{/.test(rule))

const header = (): ReturnType<typeof render> =>
  render(
    <CardHeader column="Doing" problem={null} onArchive={vi.fn()} onDelete={vi.fn()} onClose={vi.fn()} />,
  )

describe('the open card header', () => {
  it('keeps the close button in the row instead of borrowing the sign-in X', () => {
    const { container } = header()
    expect(screen.getByRole('button', { name: 'Close' })).toBeTruthy()
    expect(container.querySelector('.auth__x')).toBeNull()
    const placed = buttonRules(stylesheet()).filter((rule) => /position:\s*absolute/.test(rule))
    expect(placed).toEqual([])
  })

  it('would catch a close button taken out of the flow', () => {
    // The guard only means something if it can fail.
    expect(buttonRules('.sq26 { position: absolute; top: 10px }')).toHaveLength(1)
  })

  it('holds Archive and Delete in its menu', () => {
    header()
    fireEvent.click(screen.getByRole('button', { name: 'Card actions' }))
    const items = screen.getAllByRole('menuitem').map((item) => item.textContent)
    expect(items).toEqual(['Archive', 'Delete…'])
  })
})

describe('what a delete says', () => {
  it('wraps a checkout path inside the dialog instead of running past it', () => {
    const body = stylesheet()
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .split('}')
      .find((rule) => /(^|\s)\.ask__d\s*\{/.test(rule))
    expect(body).toMatch(/overflow-wrap:\s*anywhere/)
  })

  it('names what goes and where the checkout stays', () => {
    expect(deleteBody({ comments: 2, pinned: 1, runs: 3, checkout: '/w/card' })).toBe(
      'Its 2 comments, 1 pinned file and 3 runs go with it. The checkout and its branch stay at /w/card.',
    )
  })

  it('says nothing on disk changes when there is no checkout', () => {
    expect(deleteBody({ comments: 0, pinned: 0, runs: 0, checkout: null })).toContain('nothing on disk changes')
  })
})

describe('ending a card', () => {
  const unsaved: DeleteRefusal = { reason: '2 changes … delete anyway?', forcible: true }
  const running: DeleteRefusal = { reason: 'a run is still going on this card — stop it first', forcible: false }

  const ending = (props: Partial<Parameters<typeof CardEnding>[0]>) => {
    const hands = {
      archive: vi.fn(() => Promise.resolve<string | null>(null)),
      remove: vi.fn(() => Promise.resolve<DeleteRefusal | string | null>(null)),
      onAsk: vi.fn(),
      onProblem: vi.fn(),
      onDone: vi.fn(),
    }
    render(<CardEnding ending={{ what: 'delete' }} detail={null} {...hands} {...props} />)
    return { ...hands, ...props }
  }

  it('asks again with the reason when only unsaved work is in the way', async () => {
    const remove = vi.fn(() => Promise.resolve<DeleteRefusal | string | null>(unsaved))
    const hands = ending({ remove })
    fireEvent.click(screen.getByRole('button', { name: 'Delete' }))
    await waitFor(() => expect(hands.onAsk).toHaveBeenCalledWith({ what: 'delete', refused: unsaved }))
    expect(remove).toHaveBeenCalledWith(false)
  })

  it('presses through on the second question and closes the card', async () => {
    const remove = vi.fn(() => Promise.resolve<DeleteRefusal | string | null>(null))
    const hands = ending({ ending: { what: 'delete', refused: unsaved }, remove })
    expect(screen.getByText(unsaved.reason)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Delete anyway' }))
    await waitFor(() => expect(hands.onDone).toHaveBeenCalledWith('delete'))
    expect(remove).toHaveBeenCalledWith(true)
  })

  it('says why and offers nothing to press when a run is in the way', async () => {
    const remove = vi.fn(() => Promise.resolve<DeleteRefusal | string | null>(running))
    const hands = ending({ remove })
    fireEvent.click(screen.getByRole('button', { name: 'Delete' }))
    await waitFor(() => expect(hands.onProblem).toHaveBeenCalledWith(running.reason))
    expect(hands.onAsk).toHaveBeenCalledWith(null)
    expect(hands.onDone).not.toHaveBeenCalled()
  })

  it('asks again when archiving meets unsaved work', async () => {
    const archive = vi.fn(() => Promise.resolve<string | null>('1 change … archive anyway?'))
    const hands = ending({ ending: { what: 'archive' }, archive })
    fireEvent.click(screen.getByRole('button', { name: 'Archive' }))
    await waitFor(() =>
      expect(hands.onAsk).toHaveBeenCalledWith({ what: 'archive', refused: '1 change … archive anyway?' }),
    )
  })

  it('offers to close the terminal and stop the runs first, then archives', async () => {
    const stopLive = vi.fn(() => Promise.resolve<string | null>(null))
    const archive = vi.fn(() => Promise.resolve<string | null>(null))
    const hands = ending({ ending: { what: 'archive' }, live: { tabs: ['tab_1'], runs: ['run_1'] }, stopLive, archive })
    expect(screen.getByText(/an agent is in its terminal and 1 run is still going/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Stop it and archive' }))
    await waitFor(() => expect(hands.onDone).toHaveBeenCalledWith('archive'))
    expect(stopLive).toHaveBeenCalled()
    expect(archive).toHaveBeenCalledWith(false)
    expect(stopLive.mock.invocationCallOrder[0]!).toBeLessThan(archive.mock.invocationCallOrder[0]!)
  })

  it('says why the work could not be stopped, and ends nothing', async () => {
    const stopLive = vi.fn(() => Promise.resolve<string | null>('that run is not in flight here'))
    const hands = ending({ live: { tabs: [], runs: ['run_1'] }, stopLive })
    fireEvent.click(screen.getByRole('button', { name: 'Stop it and delete' }))
    await waitFor(() => expect(hands.onProblem).toHaveBeenCalledWith('that run is not in flight here'))
    expect(hands.remove).not.toHaveBeenCalled()
  })
})
