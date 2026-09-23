import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { CommandBlock } from '../gen/bindings'

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ writeText: vi.fn() }))
vi.mock('./live', () => ({
  ask: () => Promise.resolve({ data: null, error: null }),
  commands: {},
}))
vi.mock('./terminal', () => ({ darkNow: () => false, palette: () => ({}) }))
vi.mock('./blockRender', () => ({ plain: () => '', rendered: () => Promise.resolve([]) }))

import { BlockCard } from './BlockCard'

afterEach(cleanup)

/* Interactive, so the card asks for no output to draw. */
const block = (bookmarked: boolean): CommandBlock => ({
  id: 7,
  command: 'make build',
  cwd: '/w',
  startedAt: 0,
  endedAt: 10,
  code: 0,
  interactive: true,
  truncated: false,
  bookmarked,
})

function card(bookmarked: boolean, onBookmark = vi.fn()) {
  const { container } = render(
    <BlockCard paneId="p" block={block(bookmarked)} cols={80} home={null} jumped={false} onRerun={vi.fn()} onEdit={vi.fn()} onBookmark={onBookmark} />,
  )
  return { container, onBookmark }
}

describe('a block’s bookmark', () => {
  it('is set from the block’s menu', () => {
    const { container, onBookmark } = card(false)
    expect(container.querySelector('.blk__mark')).toBeNull()
    fireEvent.click(screen.getByLabelText('Block actions'))
    fireEvent.click(screen.getByText('Bookmark this block'))
    expect(onBookmark).toHaveBeenCalledWith(7, true)
  })

  it('shows on the header, and is taken away from the same menu', () => {
    const { container, onBookmark } = card(true)
    expect(container.querySelector('.blk__mark')).not.toBeNull()
    fireEvent.click(screen.getByLabelText('Block actions'))
    fireEvent.click(screen.getByText('Remove bookmark'))
    expect(onBookmark).toHaveBeenCalledWith(7, false)
  })
})
