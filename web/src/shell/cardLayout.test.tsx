import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { CardDescription } from './CardDescription'
import { CardLane } from './CardLane'
import { stylesheet } from './stylesheet'

afterEach(cleanup)

const moved = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    cardMove: (...args: unknown[]) => {
      moved(...args)
      return { card: {}, started: null }
    },
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn(), project: { id: 'p1' } }) }))

/** The rules inside every `@media` block with exactly this condition. */
function media(css: string, condition: string): string {
  const found: string[] = []
  const opener = `@media ${condition}`
  for (let at = css.indexOf(opener); at !== -1; at = css.indexOf(opener, at + 1)) {
    const start = css.indexOf('{', at)
    let depth = 0
    for (let i = start; i < css.length; i += 1) {
      if (css[i] === '{') depth += 1
      if (css[i] === '}') depth -= 1
      if (depth === 0) {
        found.push(css.slice(start + 1, i))
        break
      }
    }
  }
  return found.join('\n')
}

describe('the open card', () => {
  it('is two columns from 900px wide', () => {
    const rules = media(stylesheet().replace(/\/\*[\s\S]*?\*\//g, ''), '(min-width: 900px)')
    const grid = /\.cardp__grid\s*\{([^}]*)\}/.exec(rules)?.[1] ?? ''
    expect(grid).toMatch(/display:\s*grid/)
    const tracks = /grid-template-columns:\s*([^;]+)/.exec(grid)?.[1] ?? ''
    expect(tracks.replace(/\([^)]*\)/g, '').trim().split(/\s+/)).toHaveLength(2)
  })

  it('reads its description as Markdown, and writes it once clicked', () => {
    const onChange = vi.fn()
    const onDone = vi.fn()
    const { container } = render(<CardDescription body="Ship **all** of it" onChange={onChange} onDone={onDone} />)
    expect(container.querySelector('strong')?.textContent).toBe('all')
    fireEvent.click(screen.getByRole('button', { name: 'Edit the description' }))
    const field = screen.getByRole('textbox', { name: 'Description' })
    fireEvent.change(field, { target: { value: 'Ship none of it' } })
    expect(onChange).toHaveBeenCalledWith('Ship none of it')
    fireEvent.blur(field)
    expect(onDone).toHaveBeenCalled()
    expect(screen.queryByRole('textbox', { name: 'Description' })).toBeNull()
  })

  it('moves to another lane without closing, to the end of it', async () => {
    const onMoved = vi.fn()
    render(
      <CardLane
        projectId="p1"
        cardId="card_1"
        columnId="col_1"
        lanes={[
          { id: 'col_1', name: 'Todo', cards: 3 },
          { id: 'col_2', name: 'Doing', cards: 2 },
        ]}
        onMoved={onMoved}
      />,
    )
    fireEvent.change(screen.getByRole('combobox', { name: 'Lane' }), { target: { value: 'col_2' } })
    await waitFor(() => expect(onMoved).toHaveBeenCalled())
    expect(moved).toHaveBeenCalledWith('p1', 'card_1', 'col_2', 2, false)
  })
})
