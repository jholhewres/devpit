import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'

import { keptDraft, useDraft } from './useDraft'

afterEach(() => {
  cleanup()
  localStorage.clear()
})

describe('useDraft', () => {
  it('keeps unsent text for the next time the chat opens', () => {
    const first = renderHook(() => useDraft('conv_1', undefined))
    act(() => first.result.current[1]('half a thought'))
    first.unmount()

    const again = renderHook(() => useDraft('conv_1', undefined))
    expect(again.result.current[0]).toBe('half a thought')
  })

  it('forgets it once the composer is empty again', () => {
    const { result } = renderHook(() => useDraft('conv_1', undefined))
    act(() => result.current[1]('sent soon'))
    act(() => result.current[1](''))
    expect(keptDraft('conv_1')).toBeNull()
  })

  it('lets a card’s draft win over an old one', () => {
    localStorage.setItem('devpit.chatDraft.conv_1', 'old')
    const { result } = renderHook(() => useDraft('conv_1', 'from the card'))
    expect(result.current[0]).toBe('from the card')
  })
})
