import { renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { useClosing } from './useClosing'
import { useOverlays } from './useOverlays'
import { useProjects } from './useProjects'
import { useTabs } from './useTabs'
import { useWidths } from './useWidths'

/*
 * The shell's context is built from these hooks, and its memo holds only if
 * each of them hands back the same object when nothing changed. A fresh
 * object from any one of them re-rendered every screen in the app — hidden
 * tabs included — on every render of the provider.
 */
describe('the hooks the shell is built from', () => {
  it('hand back the same object across a render that changed nothing', () => {
    const open: never[] = []
    const closeNow = (): void => {}
    const running: never[] = []
    const unsaved = new Set<string>()
    const hooks: (() => unknown)[] = [
      () => useClosing({ open, closeNow, running, unsaved }),
      () => useWidths(),
      () => useOverlays(),
      () => useProjects(),
      () => useTabs(null),
    ]
    for (const hook of hooks) {
      const { result, rerender } = renderHook(hook)
      const first = result.current
      rerender()
      expect(result.current).toBe(first)
    }
  })
})
