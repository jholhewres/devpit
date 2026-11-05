import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Attachment, ChatQueue } from '../gen/bindings'
import { EMPTY, ended, handedBack, merged, raised } from './queueing'
import { useQueue, type Talk } from './useQueue'

/* The queue files on disk, by project and conversation. */
const disk = new Map<string, ChatQueue>()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    chatQueue: async (projectId: string, id: string) => disk.get(`${projectId}/${id}`) ?? { items: [], paused: false },
    chatQueueKeep: async (projectId: string, id: string, queue: ChatQueue) => (disk.set(`${projectId}/${id}`, queue), queue),
  },
}))

afterEach(() => {
  cleanup()
  disk.clear()
})

const none = { files: [], skills: [] }
const picture: Attachment = { name: 'pasted-1.png', path: '/h/pasted/pasted-1.png', kind: 'png' }

/* A queue on a chat that is sending or not, read from disk before the test goes on. */
async function queueOn(talk: Talk) {
  const hook = renderHook((props: Talk) => useQueue('p1', 'conv_1', props), { initialProps: talk })
  await act(async () => {})
  return hook
}

describe('messages typed while a turn runs', () => {
  it('wait for the turn, then go one at a time, in order, each with its own files', async () => {
    const say = vi.fn()
    const { result, rerender } = await queueOn({ sending: true, say })
    act(() => {
      result.current.add('first', none)
      result.current.add('second', { files: [picture], skills: ['review'] })
    })
    expect(say).not.toHaveBeenCalled()
    rerender({ sending: false, say })
    expect(say).toHaveBeenCalledTimes(1)
    expect(say.mock.calls[0]![0]).toBe('first')
    expect(say.mock.calls[0]![2]).toMatchObject({ files: [] })
    // The first one started a turn; the second waits for it.
    rerender({ sending: true, say })
    expect(result.current.waiting.map((one) => one.prompt)).toEqual(['second'])
    rerender({ sending: false, say })
    expect(say.mock.calls[1]![0]).toBe('second')
    expect(say.mock.calls[1]![2]).toMatchObject({ files: [picture], skills: ['review'] })
  })

  it('stopped, are held and nothing goes by itself; resuming sends the first one', async () => {
    const say = vi.fn()
    const { result, rerender } = await queueOn({ sending: true, say })
    act(() => {
      result.current.add('a', { files: [picture], skills: [] })
      result.current.add('b', none)
    })
    act(() => result.current.pause())
    rerender({ sending: false, say })
    expect(say).not.toHaveBeenCalled()
    expect(result.current.paused).toBe(true)
    expect(result.current.waiting).toHaveLength(2)

    act(() => result.current.resume())
    expect(say).toHaveBeenCalledTimes(1)
    expect(say.mock.calls[0]![0]).toBe('a')
    expect(say.mock.calls[0]![2]).toMatchObject({ files: [picture] })
    expect(result.current.paused).toBe(false)
    expect(result.current.waiting.map((one) => one.prompt)).toEqual(['b'])
  })

  it('are held by a turn that failed, too', async () => {
    const say = vi.fn()
    const { result, rerender } = await queueOn({ sending: true, say })
    act(() => result.current.add('a', none))
    rerender({ sending: false, failed: true, say })
    expect(say).not.toHaveBeenCalled()
    expect(result.current.paused).toBe(true)
  })

  it('go into the running turn now, by which one and not by its words, and stay when the turn would not take it', async () => {
    const say = vi.fn()
    let takes = false
    const steer = vi.fn(async () => takes)
    const { result } = await queueOn({ sending: true, say, steer })
    act(() => {
      result.current.add('same', none)
      result.current.add('same', { files: [picture], skills: [] })
    })
    const [, second] = result.current.waiting
    await act(async () => result.current.now(second!.id))
    expect(steer).toHaveBeenLastCalledWith('same', second)
    expect(result.current.waiting).toHaveLength(2)
    takes = true
    await act(async () => result.current.now(second!.id))
    expect(result.current.waiting.map((one) => one.files)).toEqual([[]])
    expect(say).not.toHaveBeenCalled()
  })

  it('go back to the composer as one draft, with every file once', async () => {
    const { result } = await queueOn({ sending: true, say: vi.fn() })
    act(() => {
      result.current.add('a', { files: [picture], skills: ['x'] })
      result.current.add('b', { files: [picture], skills: ['y'] })
    })
    let back: ReturnType<typeof result.current.takeAll> = null
    act(() => {
      back = result.current.takeAll()
    })
    expect(back).toEqual({ prompt: 'a\n\nb', files: [picture], skills: ['x', 'y'] })
    expect(result.current.waiting).toEqual([])
  })
})

describe('a queue kept on disk', () => {
  it('three queued, one with a picture, are there in order after the tab closes and opens again', async () => {
    const say = vi.fn()
    const first = await queueOn({ sending: true, say })
    act(() => {
      first.result.current.add('one', none)
      first.result.current.add('two', { files: [picture], skills: [] })
      first.result.current.add('three', none)
    })
    await waitFor(() => expect(disk.get('p1/conv_1')?.items).toHaveLength(3))
    first.unmount()

    const again = await queueOn({ sending: false, say })
    await waitFor(() => expect(again.result.current.waiting).toHaveLength(3))
    expect(again.result.current.waiting.map((one) => one.prompt)).toEqual(['one', 'two', 'three'])
    expect(again.result.current.waiting.map((one) => one.files)).toEqual([[], [picture], []])
    expect(say).not.toHaveBeenCalled()
  })

  it('emptied, is written empty', async () => {
    const { result, rerender } = await queueOn({ sending: true, say: vi.fn() })
    act(() => result.current.add('one', none))
    await waitFor(() => expect(disk.get('p1/conv_1')?.items).toHaveLength(1))
    rerender({ sending: false, say: vi.fn() })
    await waitFor(() => expect(disk.get('p1/conv_1')?.items).toEqual([]))
  })
})

describe('the rules of a queue', () => {
  const item = (id: string) => ({ id, prompt: id, files: [], skills: [] })
  const of = (...ids: string[]): ChatQueue => ({ items: ids.map(item), paused: false })

  it('a held queue that empties holds nothing back any more', () => {
    expect(ended({ items: [item('a')], paused: true }, false).queue.paused).toBe(true)
    expect(ended(EMPTY, true)).toEqual({ next: null, queue: EMPTY })
    expect(handedBack([]).prompt).toBe('')
  })

  it('moving one up swaps it with the one before, and the first stays first', () => {
    expect(raised(of('a', 'b', 'c'), 'c').items.map((one) => one.id)).toEqual(['a', 'c', 'b'])
    expect(raised(of('a', 'b'), 'a').items.map((one) => one.id)).toEqual(['a', 'b'])
  })

  it('what was queued before the disk answered goes after what the disk kept', () => {
    expect(merged(of('a'), of('b', 'a')).items.map((one) => one.id)).toEqual(['a', 'b'])
    const kept = of('a')
    expect(merged(kept, of())).toBe(kept)
  })
})
