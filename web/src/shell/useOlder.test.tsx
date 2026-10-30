import { act, cleanup, render, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { PAGE, useOlder } from './useOlder'

const asked: unknown[][] = []
const pages: { messages: Message[]; older: boolean }[] = []
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatHistory: (...args: unknown[]) => (asked.push(args), { messages: pages.shift()?.messages ?? [], older: false }),
  },
}))

const message = (id: string): Message =>
  ({ id, turnId: null, role: 'assistant', parts: [], createdAt: 0, streaming: false }) as Message

/* The top of the thread coming into view, as the browser would say it. */
let seen: (entries: { isIntersecting: boolean }[]) => void = () => {}
beforeEach(() => {
  vi.stubGlobal(
    'IntersectionObserver',
    class {
      constructor(callback: typeof seen) {
        seen = callback
      }
      observe(): void {}
      disconnect(): void {}
    },
  )
})
afterEach(() => {
  cleanup()
  seen = () => {}
  vi.unstubAllGlobals()
  asked.length = 0
  pages.length = 0
})

function thread(first: Message[], older: boolean): { box: HTMLDivElement; shown: () => string[] } {
  let messages = first
  let more = older
  let height = 1000
  let redraw: () => void = () => {}
  function Thread(): React.JSX.Element {
    const top = useOlder(
      { projectId: 'p', conversationId: 'c' },
      messages,
      more,
      (page, left) => {
        messages = [...page, ...messages]
        more = left
        height += 400 * page.length
        redraw()
      },
      boxRef,
    )
    return (
      <div ref={boxRef}>
        <div ref={top} />
        {messages.map((one) => (
          <p key={one.id}>{one.id}</p>
        ))}
      </div>
    )
  }
  const boxRef = { current: null as HTMLDivElement | null }
  const { container, rerender } = render(<Thread />)
  redraw = () => rerender(<Thread />)
  const box = container.firstElementChild as HTMLDivElement
  let top = 0
  Object.defineProperty(box, 'clientHeight', { value: 200 })
  Object.defineProperty(box, 'scrollHeight', { get: () => height })
  Object.defineProperty(box, 'scrollTop', { get: () => top, set: (to: number) => (top = to) })
  return {
    box,
    shown: () => Array.from(box.querySelectorAll('p')).map((one) => one.textContent ?? ''),
  }
}

describe("a chat's older messages", () => {
  it('are asked for, a page before the first one drawn, when the top comes into view', async () => {
    pages.push({ messages: [message('m1'), message('m2')], older: false })
    const { shown } = thread([message('m3'), message('m4')], true)
    await act(async () => seen([{ isIntersecting: true }]))
    await waitFor(() => expect(shown()).toEqual(['m1', 'm2', 'm3', 'm4']))
    expect(asked).toEqual([['p', 'c', PAGE, 'm3']])
  })

  it('go in above without moving what was on screen', async () => {
    pages.push({ messages: [message('m1'), message('m2')], older: false })
    const { box } = thread([message('m3'), message('m4')], true)
    box.scrollTop = 0
    await act(async () => seen([{ isIntersecting: true }]))
    // 800 px went in above: what was at the top is now 800 px down.
    await waitFor(() => expect(box.scrollTop).toBe(800))
  })

  it('are not asked for when none are left', async () => {
    thread([message('m1')], false)
    await act(async () => seen([{ isIntersecting: true }]))
    expect(asked).toEqual([])
  })
})
