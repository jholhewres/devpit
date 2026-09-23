import { describe, expect, it, vi } from 'vitest'

import { claimMenu, clamped, menuFocus, menuPoint } from './menuRules'

describe('one menu at a time', () => {
  it('closes the menu already open when another opens', () => {
    const first = vi.fn()
    const second = vi.fn()
    claimMenu(first)
    const release = claimMenu(second)
    expect(first).toHaveBeenCalledOnce()
    expect(second).not.toHaveBeenCalled()
    release()
    const last = claimMenu(vi.fn())
    expect(second).not.toHaveBeenCalled()
    last()
  })
})

describe('where a menu goes', () => {
  it('stays inside the window on both axes', () => {
    const view = { width: 800, height: 600 }
    expect(clamped({ x: 790, y: 590 }, { width: 200, height: 300 }, view)).toEqual({ left: 592, top: 292 })
    expect(clamped({ x: -5, y: -5 }, { width: 200, height: 300 }, view)).toEqual({ left: 8, top: 8 })
  })

  it('opens under the element a keyboard asked on, not at the corner', () => {
    const row = document.createElement('div')
    row.getBoundingClientRect = () => ({ left: 40, bottom: 120 }) as DOMRect
    expect(menuPoint({ clientX: 0, clientY: 0, target: row })).toEqual({ x: 40, y: 120 })
  })

  it('is walked with the arrows, Home and End', () => {
    expect(menuFocus('ArrowDown', 0, 3)).toBe(1)
    expect(menuFocus('ArrowUp', 1, 3)).toBe(0)
    expect(menuFocus('End', 0, 3)).toBe(2)
    expect(menuFocus('x', 0, 3)).toBeNull()
  })
})
