import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { useState } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { ColumnDeleted } from '../gen/bindings'
import { LaneMenu, moveTitle } from './LaneMenuView'

afterEach(cleanup)

const others = [
  { id: 'col_2', name: 'Doing' },
  { id: 'col_3', name: 'Done' },
]

const answers = (...said: ColumnDeleted[]) => {
  const queue = [...said]
  return vi.fn((_moveTo: string | null) => Promise.resolve<ColumnDeleted | null>(queue.shift() ?? null))
}

function Menu({ onDelete }: { onDelete: (moveTo: string | null) => Promise<ColumnDeleted | null> }): React.JSX.Element {
  const [open, setOpen] = useState(false)
  return (
    <LaneMenu
      name="Todo"
      others={others}
      open={open}
      onOpen={setOpen}
      onRename={vi.fn()}
      onAddCard={vi.fn()}
      onShift={vi.fn()}
      onDelete={onDelete}
    />
  )
}

const askToDelete = (): void => {
  fireEvent.click(screen.getByRole('button', { name: 'Todo actions' }))
  fireEvent.click(screen.getByRole('menuitem', { name: 'Delete…' }))
  fireEvent.click(screen.getByRole('button', { name: 'Delete' }))
}

describe('deleting a lane', () => {
  it('says how many cards the backend found, and moves them where chosen', async () => {
    const onDelete = answers({ deleted: false, cardsInTheWay: 3 }, { deleted: true, cardsInTheWay: 0 })
    render(<Menu onDelete={onDelete} />)

    askToDelete()
    expect(await screen.findByText('Move 3 cards out of “Todo” first')).toBeTruthy()
    expect(onDelete).toHaveBeenCalledWith(null)

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'col_3' } })
    fireEvent.click(screen.getByRole('button', { name: 'Move and delete' }))
    await waitFor(() => expect(onDelete).toHaveBeenLastCalledWith('col_3'))
    expect(document.body.textContent).not.toContain('go too')
  })

  it('does not ask where cards go when the lane was empty', async () => {
    const onDelete = answers({ deleted: true, cardsInTheWay: 0 })
    render(<Menu onDelete={onDelete} />)
    askToDelete()
    await waitFor(() => expect(onDelete).toHaveBeenCalledTimes(1))
    expect(screen.queryByRole('button', { name: 'Move and delete' })).toBeNull()
  })

  it('counts one card as one', () => {
    expect(moveTitle(1, 'Todo')).toBe('Move 1 card out of “Todo” first')
  })
})
