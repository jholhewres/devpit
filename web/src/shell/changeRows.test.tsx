import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Change } from '../gen/bindings'
import { ChangeRows } from './ChangeRows'
import { shapeFor } from './FileGlyph'

afterEach(cleanup)

const change = (path: string, over: Partial<Change> = {}): Change => ({
  path,
  status: 'modified',
  added: 1,
  removed: 0,
  staged: false,
  ...over,
})

function show(changes: readonly Change[], over: { staged?: boolean } = {}) {
  const opened = vi.fn()
  const stagedPaths = vi.fn()
  const discarded = vi.fn()
  const view = render(
    <ChangeRows
      changes={changes}
      staged={over.staged ?? false}
      busy={false}
      onOpen={opened}
      onStage={stagedPaths}
      onDiscard={discarded}
    />,
  )
  return { view, opened, stagedPaths, discarded }
}

/* The panel with forty files in it. A flat list of basenames is four `mod.rs`
   rows that say nothing; a flat list of whole paths is a column of repeated
   prefixes in something the width of a sidebar. */

describe('what the panel draws', () => {
  it('shows a file by its own name, with the folder above it', () => {
    show([change('apps/desktop/src/main.rs')])
    expect(screen.getByText('apps/desktop/src')).toBeTruthy()
    expect(screen.getByText('main.rs')).toBeTruthy()
  })

  it('counts the files under a folder', () => {
    show([change('a/one.rs'), change('a/two.rs')])
    expect(screen.getByText('2')).toBeTruthy()
  })

  it('opens the diff and not the file when a row is clicked', () => {
    // The question the panel answers is what changed, and the file alone does
    // not answer it.
    const { opened } = show([change('a/one.rs')])
    fireEvent.click(screen.getByText('one.rs'))
    expect(opened).toHaveBeenCalledWith('a/one.rs')
  })

  it('stages one file by its whole path, which is what git takes', () => {
    const { stagedPaths } = show([change('deep/down/one.rs')])
    fireEvent.click(screen.getByLabelText('Stage deep/down/one.rs'))
    expect(stagedPaths).toHaveBeenCalledWith(['deep/down/one.rs'])
  })

  it('says unstage on a row that is already in the index', () => {
    show([change('a/one.rs', { staged: true })], { staged: true })
    expect(screen.getByLabelText('Unstage a/one.rs')).toBeTruthy()
  })
})

describe('a folder is one decision', () => {
  it('stages everything under it in one click', () => {
    // Staging nine files one at a time is nine clicks for one decision.
    const { stagedPaths } = show([
      change('a/b/one.rs'),
      change('a/two.rs'),
      change('z.rs'),
    ])
    fireEvent.click(screen.getByLabelText('Stage everything in a'))
    expect(stagedPaths.mock.calls[0]![0].sort()).toEqual(['a/b/one.rs', 'a/two.rs'])
  })

  it('closes and opens', () => {
    show([change('a/one.rs')])
    const folder = screen.getByText('a').closest('button')!
    expect(folder.getAttribute('aria-expanded')).toBe('true')
    expect(screen.queryByText('one.rs')).toBeTruthy()

    fireEvent.click(folder)
    expect(folder.getAttribute('aria-expanded')).toBe('false')
    expect(screen.queryByText('one.rs')).toBeNull()
  })

  it('opens everything by default, because that is what the panel is for', () => {
    show([change('a/b/c/one.rs'), change('x/two.rs')])
    expect(screen.getByText('one.rs')).toBeTruthy()
    expect(screen.getByText('two.rs')).toBeTruthy()
  })

  it('collapses the lot, and puts them back', () => {
    show([change('a/one.rs'), change('b/two.rs')])
    fireEvent.click(screen.getByText('Collapse all'))
    expect(screen.queryByText('one.rs')).toBeNull()
    fireEvent.click(screen.getByText('Expand all'))
    expect(screen.getByText('one.rs')).toBeTruthy()
  })

  it('does not offer to collapse a tree with nothing to collapse', () => {
    show([change('one.rs'), change('two.rs')])
    expect(screen.queryByText('Collapse all')).toBeNull()
  })
})

describe('the row itself', () => {
  it('marks a file with a shape for its kind', () => {
    const { view } = show([change('a/main.rs')])
    expect(view.container.querySelector('.gitrow__ico')).toBeTruthy()
  })

  it('colours that shape by what git says happened', () => {
    // Shape says what kind of file; colour says what happened to it. Two
    // marks would be two messages competing on one glyph.
    const { view } = show([change('a/new.rs', { status: 'untracked' })])
    expect(view.container.querySelector('.row__g--untracked .gitrow__ico')).toBeTruthy()
  })

  it('says where a file is exactly once', () => {
    // The tree is the answer to "where". Repeating the folder on the file row
    // is the same words twice on two lines that touch.
    show([change('crates/core/src/mod.rs')])
    expect(screen.getAllByText('crates/core/src')).toHaveLength(1)
    expect(screen.getByText('mod.rs')).toBeTruthy()
  })

  it('keeps its actions out of the way until the row is wanted', () => {
    // Eighty buttons competing with forty names is a panel nobody can read.
    const { view } = show([change('a/one.rs')])
    const file = view.container.querySelector('.gitrow--file .gitrow__does')
    expect(file).toBeTruthy()
    expect(file!.querySelectorAll('button')).toHaveLength(2)
    // The folder keeps one: it has nothing of its own to discard.
    const folder = view.container.querySelector('.gitrow--dir .gitrow__does')
    expect(folder!.querySelectorAll('button')).toHaveLength(1)
  })
})

describe('the shapes', () => {
  it.each([
    ['main.rs', 'code'],
    ['App.tsx', 'code'],
    ['index.html', 'markup'],
    ['shell.css', 'style'],
    ['package.json', 'data'],
    ['Cargo.toml', 'config'],
    ['Dockerfile', 'config'],
    ['README.md', 'doc'],
    ['icon.png', 'image'],
    ['LICENSE', 'doc'],
  ])('reads %s as %s', (path, shape) => {
    expect(shapeFor(path)).toBe(shape)
  })

  it('comes from the same table that colours the file', () => {
    // One table, two readers: a new extension arrives in both at once.
    expect(shapeFor('a.zig')).toBe('code')
    expect(shapeFor('a.yaml')).toBe('config')
  })
})
