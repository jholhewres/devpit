import { act, render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { Shell } from './shape'
import { createShellStore, shallowEqual, ShellStoreContext, useShellPick } from './shellStore'

/* The point of the store: a component reading one slice renders again only
   when that slice changes, not on every change to the shell. */
describe('useShellPick', () => {
  it('renders again only when what it picked changed', () => {
    const show = (): void => {}
    const first = { show, side: true } as unknown as Shell
    const store = createShellStore(first)
    let renders = 0
    function Link(): null {
      useShellPick((shell) => shell.show)
      renders += 1
      return null
    }
    render(
      <ShellStoreContext.Provider value={store}>
        <Link />
      </ShellStoreContext.Provider>,
    )
    expect(renders).toBe(1)

    act(() => {
      store.put({ show, side: false } as unknown as Shell)
      store.tell()
    })
    expect(renders).toBe(1)

    act(() => {
      store.put({ show: () => {}, side: false } as unknown as Shell)
      store.tell()
    })
    expect(renders).toBe(2)
  })

  it('compares an object it picked field by field', () => {
    expect(shallowEqual({ a: 1, b: 'x' }, { a: 1, b: 'x' })).toBe(true)
    expect(shallowEqual({ a: 1 }, { a: 2 })).toBe(false)
    expect(shallowEqual({ a: 1 }, { a: 1, b: 2 })).toBe(false)
  })
})
