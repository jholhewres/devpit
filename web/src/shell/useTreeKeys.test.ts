import { afterEach, describe, expect, it, vi } from 'vitest'

import { arrowAction, onTreeKeyDown, rove, type RowKind } from './useTreeKeys'

describe('the left/right rule on a tree row', () => {
  it('opens a closed folder on the right, but steps into one already open', () => {
    expect(arrowAction('right', 'closed-folder')).toBe('open')
    expect(arrowAction('right', 'open-folder')).toBe('first-child')
  })

  it('does nothing on the right for a file — there is nothing to open', () => {
    expect(arrowAction('right', 'file')).toBe('none')
  })

  it('closes an open folder on the left', () => {
    expect(arrowAction('left', 'open-folder')).toBe('close')
  })

  it('steps to the parent on the left for a closed folder and for a file alike', () => {
    expect(arrowAction('left', 'closed-folder')).toBe('parent')
    expect(arrowAction('left', 'file')).toBe('parent')
  })

  /* All three kinds, in both directions — a rule that only checked two of
     them would still pass if the third silently inherited another kind's
     behaviour. Breaking either inequality below by hand (say, folding
     closed-folder into file) makes this fail. */
  it('gives all three kinds a say in both directions', () => {
    const kinds: RowKind[] = ['open-folder', 'closed-folder', 'file']
    const right = kinds.map((kind) => arrowAction('right', kind))
    const left = kinds.map((kind) => arrowAction('left', kind))

    expect(right[1]).not.toBe(right[2]) // closed folder opens; a file does nothing
    expect(left[0]).not.toBe(left[1]) // an open folder closes; a closed one climbs out
    expect(left[0]).not.toBe(left[2]) // an open folder closes; a file climbs out
  })
})

describe('moving focus across the visible rows', () => {
  afterEach(() => {
    document.body.innerHTML = ''
  })

  // jsdom only tracks `activeElement` for a node connected to the document,
  // so the container — not just each row — has to live under `<body>`.
  function makeContainer(): HTMLDivElement {
    const container = document.createElement('div')
    document.body.append(container)
    return container
  }

  function makeRow(kind: 'folder' | 'file', depth: number, open = false): HTMLButtonElement {
    const row = document.createElement('button')
    row.className = 'row'
    row.dataset.kind = kind
    row.dataset.depth = String(depth)
    row.tabIndex = -1 // matches Row.tsx's own markup — `rove` is what hands one row a 0
    if (kind === 'folder') row.setAttribute('aria-expanded', String(open))
    return row
  }

  function fire(target: HTMLButtonElement, container: HTMLElement, key: string): boolean {
    let prevented = false
    const fake = {
      key,
      target,
      currentTarget: container,
      preventDefault: () => {
        prevented = true
      },
    } as unknown as Parameters<typeof onTreeKeyDown>[0]
    onTreeKeyDown(fake)
    return prevented
  }

  it('steps Down and Up between sibling rows', () => {
    const container = makeContainer()
    const a = makeRow('file', 0)
    const b = makeRow('file', 0)
    container.append(a, b)

    fire(a, container, 'ArrowDown')
    expect(document.activeElement).toBe(b)

    fire(b, container, 'ArrowUp')
    expect(document.activeElement).toBe(a)
  })

  it('jumps to the first and last row on Home and End', () => {
    const container = makeContainer()
    const first = makeRow('file', 0)
    const middle = makeRow('file', 0)
    const last = makeRow('file', 0)
    container.append(first, middle, last)

    fire(middle, container, 'End')
    expect(document.activeElement).toBe(last)

    fire(middle, container, 'Home')
    expect(document.activeElement).toBe(first)
  })

  it('opens a closed folder on the right by clicking it, without moving focus', () => {
    const container = makeContainer()
    const folder = makeRow('folder', 0, false)
    const click = vi.fn()
    folder.onclick = click
    container.append(folder)
    folder.focus() // a keydown only ever fires on the element already focused

    fire(folder, container, 'ArrowRight')
    expect(click).toHaveBeenCalledOnce()
    expect(document.activeElement).toBe(folder)
  })

  it('steps into an open folder on the right by moving to its first child', () => {
    const container = makeContainer()
    const folder = makeRow('folder', 0, true)
    const click = vi.fn()
    folder.onclick = click
    const child = makeRow('file', 1)
    container.append(folder, child)

    fire(folder, container, 'ArrowRight')
    expect(click).not.toHaveBeenCalled()
    expect(document.activeElement).toBe(child)
  })

  it('does nothing on the right for an open folder with no children — an empty `Some`, not a sibling to walk to', () => {
    const container = makeContainer()
    const folder = makeRow('folder', 0, true)
    const click = vi.fn()
    folder.onclick = click
    const sibling = makeRow('file', 0) // same depth: not this folder's child
    container.append(folder, sibling)
    folder.focus()

    fire(folder, container, 'ArrowRight')
    expect(click).not.toHaveBeenCalled()
    expect(document.activeElement).toBe(folder)
  })

  it('closes an open folder on the left by clicking it, without moving focus', () => {
    const container = makeContainer()
    const folder = makeRow('folder', 0, true)
    const click = vi.fn()
    folder.onclick = click
    container.append(folder)
    folder.focus() // a keydown only ever fires on the element already focused

    fire(folder, container, 'ArrowLeft')
    expect(click).toHaveBeenCalledOnce()
    expect(document.activeElement).toBe(folder)
  })

  it('steps a file out to its parent folder on the left', () => {
    const container = makeContainer()
    const folder = makeRow('folder', 0, true)
    const child = makeRow('file', 1)
    const grandchild = makeRow('file', 2)
    container.append(folder, child, grandchild)

    fire(grandchild, container, 'ArrowLeft')
    expect(document.activeElement).toBe(child)

    fire(child, container, 'ArrowLeft')
    expect(document.activeElement).toBe(folder)
  })

  it('leaves every other key alone', () => {
    const container = makeContainer()
    const a = makeRow('file', 0)
    container.append(a)
    a.focus()

    expect(fire(a, container, 'a')).toBe(false)
    expect(document.activeElement).toBe(a)
  })

  describe('the tree as one tab stop', () => {
    // A MutationObserver callback runs as a microtask; flushing one lets it run.
    const settle = (): Promise<void> => new Promise((resolve) => queueMicrotask(resolve))

    it('gives only the first row a tab stop before anything has been focused', async () => {
      const container = makeContainer()
      const a = makeRow('file', 0)
      const b = makeRow('file', 0)
      container.append(a, b)
      const stop = rove(container)
      await settle()

      expect(a.tabIndex).toBe(0)
      expect(b.tabIndex).toBe(-1)
      stop()
    })

    it('moves the tab stop to whichever row is focused', () => {
      const container = makeContainer()
      const a = makeRow('file', 0)
      const b = makeRow('file', 0)
      container.append(a, b)
      const stop = rove(container)

      b.focus()
      expect(b.tabIndex).toBe(0)
      expect(a.tabIndex).toBe(-1)
      stop()
    })

    it('hands the stop back when the row that held it disappears', async () => {
      const container = makeContainer()
      const a = makeRow('file', 0)
      const b = makeRow('file', 0)
      container.append(a, b)
      const stop = rove(container)

      b.focus()
      expect(b.tabIndex).toBe(0)
      b.remove()
      await settle()

      expect(a.tabIndex).toBe(0)
      stop()
    })

    it('stops watching once torn down', async () => {
      const container = makeContainer()
      const a = makeRow('file', 0)
      container.append(a)
      const stop = rove(container)
      await settle()
      stop()

      const b = makeRow('file', 0)
      container.append(b)
      await settle()

      // No observer left to hand `b` a tab stop of its own.
      expect(b.tabIndex).toBe(-1)
    })
  })
})
