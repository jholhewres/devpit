import { cleanup, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { blocks } from './markdown'
import { Markdown } from './MarkdownView'

vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn() }) }))

afterEach(cleanup)

const TABLE = [
  '| Ideia | O que é | Dados |',
  '|---|:---:|--:|',
  '| **Vault** | Markdown com `wikilinks` | `.md` |',
  '| Notas | curta |',
].join('\n')

describe('a markdown table', () => {
  it('is read as a header, its alignment and rows as wide as the header', () => {
    expect(blocks(TABLE)).toEqual([
      {
        kind: 'table',
        head: ['Ideia', 'O que é', 'Dados'],
        align: [null, 'center', 'right'],
        rows: [
          ['**Vault**', 'Markdown com `wikilinks`', '`.md`'],
          ['Notas', 'curta', ''],
        ],
      },
    ])
  })

  it('keeps an escaped pipe inside its cell', () => {
    const [table] = blocks('| a | b |\n|---|---|\n| `x\\|y` | z |')
    expect(table).toMatchObject({ rows: [['`x|y`', 'z']] })
  })

  /* A pipe in prose is not a table: without the delimiter row under it, or
     with a delimiter that has a different number of cells, it stays text. */
  it('leaves a line with pipes but no delimiter row as a paragraph', () => {
    expect(blocks('a | b\nstill prose')[0]?.kind).toBe('paragraph')
    expect(blocks('| a | b |\n|---|\n| 1 | 2 |')[0]?.kind).toBe('paragraph')
  })

  it('starts right under a line of prose and ends at a blank line', () => {
    const read = blocks(`Intro line\n${TABLE}\n\nAfter`)
    expect(read.map((block) => block.kind)).toEqual(['paragraph', 'table', 'paragraph'])
  })

  it('draws cells with the same inline marks as the rest of the document', () => {
    const { container } = render(<Markdown source={TABLE} />)
    const heads = [...container.querySelectorAll('th')].map((th) => th.textContent)
    expect(heads).toEqual(['Ideia', 'O que é', 'Dados'])
    expect(container.querySelector('td strong')?.textContent).toBe('Vault')
    expect(container.querySelector('td code.md__tick')?.textContent).toBe('wikilinks')
    expect((container.querySelectorAll('td')[2] as HTMLElement).style.textAlign).toBe('right')
  })
})
