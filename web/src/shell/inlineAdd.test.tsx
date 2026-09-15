import { readdirSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { useState } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { NewColumn } from './BoardToolbar'
import { LaneFoot } from './Lane'

afterEach(cleanup)

function Foot({ onAddCard, onAdding }: { onAddCard: (title: string) => void; onAdding: (open: boolean) => void }): React.JSX.Element {
  const [adding, setAdding] = useState(false)
  return (
    <LaneFoot
      adding={adding}
      onAdding={(open) => {
        onAdding(open)
        setAdding(open)
      }}
      onAddCard={onAddCard}
    />
  )
}

const type = (field: HTMLElement, text: string): void => {
  fireEvent.change(field, { target: { value: text } })
  fireEvent.keyDown(field, { key: 'Enter' })
}

describe('adding a card in place', () => {
  it('makes one card per Enter with the title typed, and stays for the next', () => {
    const onAddCard = vi.fn()
    render(<Foot onAddCard={onAddCard} onAdding={vi.fn()} />)
    fireEvent.click(screen.getByRole('button', { name: '+ Add card' }))
    const field = screen.getByRole('textbox', { name: 'New card title' })

    type(field, 'First')
    type(screen.getByRole('textbox', { name: 'New card title' }), 'Second')
    expect(onAddCard.mock.calls).toEqual([['First'], ['Second']])
    expect((screen.getByRole('textbox', { name: 'New card title' }) as HTMLInputElement).value).toBe('')
  })

  it('makes nothing from an empty Enter', () => {
    const onAddCard = vi.fn()
    render(<Foot onAddCard={onAddCard} onAdding={vi.fn()} />)
    fireEvent.click(screen.getByRole('button', { name: '+ Add card' }))
    type(screen.getByRole('textbox', { name: 'New card title' }), '   ')
    expect(onAddCard).not.toHaveBeenCalled()
  })

  it('goes away on Escape, or when left empty, but not when left with a title in it', () => {
    const onAdding = vi.fn()
    render(<Foot onAddCard={vi.fn()} onAdding={onAdding} />)
    fireEvent.click(screen.getByRole('button', { name: '+ Add card' }))
    const field = screen.getByRole('textbox', { name: 'New card title' })
    fireEvent.change(field, { target: { value: 'Half a title' } })
    fireEvent.blur(field)
    expect(onAdding).toHaveBeenLastCalledWith(true)

    fireEvent.keyDown(field, { key: 'Escape' })
    expect(onAdding).toHaveBeenLastCalledWith(false)
    expect(screen.getByRole('button', { name: '+ Add card' })).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: '+ Add card' }))
    fireEvent.blur(screen.getByRole('textbox', { name: 'New card title' }))
    expect(onAdding).toHaveBeenLastCalledWith(false)
  })
})

describe('adding a column in place', () => {
  it('takes the name typed and stays for the next', () => {
    const onAdd = vi.fn()
    render(<NewColumn onAdd={onAdd} />)
    fireEvent.click(screen.getByRole('button', { name: '+ Column' }))
    type(screen.getByRole('textbox', { name: 'New column name' }), 'Review')
    expect(onAdd).toHaveBeenCalledWith('Review')
    expect(screen.getByRole('textbox', { name: 'New column name' })).toBeTruthy()
    fireEvent.keyDown(screen.getByRole('textbox', { name: 'New column name' }), { key: 'Escape' })
    expect(screen.getByRole('button', { name: '+ Column' })).toBeTruthy()
  })
})

describe('placeholder names', () => {
  it('are no longer written into the board by anything', () => {
    const src = resolve(process.cwd(), 'src')
    const said = (readdirSync(src, { recursive: true }) as string[])
      .filter((path) => /\.tsx?$/.test(path) && !/\.test\.tsx?$/.test(path))
      .filter((path) => /'New card'|'New column'/.test(readFileSync(resolve(src, path), 'utf8')))
    expect(said).toEqual([])
  })
})
