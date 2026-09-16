import { afterEach, describe, expect, it, vi } from 'vitest'

import { drawOnTheGpu } from './terminalAddons'

/* jsdom gives a canvas no context at all, which is the same answer a headless
   machine gives — so the context has to be stood in for to reach the path that
   only runs when there is one. */
const withAContext = (context: unknown): void => {
  HTMLCanvasElement.prototype.getContext = vi.fn(() => context) as never
}

afterEach(() => vi.restoreAllMocks())

/* A pane that cannot get a GPU context has to keep working. The DOM renderer
   is what every pane used before this, so falling back to it is not a
   degraded mode — it is the old one. */
describe('the renderer a pane draws with', () => {
  const terminal = (): { loadAddon: ReturnType<typeof vi.fn> } => ({ loadAddon: vi.fn() })

  it('loads the GPU renderer and watches for the context going away', () => {
    withAContext({})
    const onContextLoss = vi.fn()
    const pane = terminal()
    const took = drawOnTheGpu(pane as never, () => ({ dispose: vi.fn(), onContextLoss }) as never)

    expect(took).toBe(true)
    expect(pane.loadAddon).toHaveBeenCalledOnce()
    expect(onContextLoss).toHaveBeenCalledOnce()
  })

  it('says so and loads nothing when the addon will not start', () => {
    withAContext({})
    const pane = terminal()
    const took = drawOnTheGpu(pane as never, () => {
      throw new Error('no WebGL here')
    })

    expect(took).toBe(false)
    expect(pane.loadAddon).not.toHaveBeenCalled()
  })

  /* The loss handler is the half that is easy to forget: an addon left loaded
     after its context is gone draws nothing at all, and a blank pane is worse
     than a slow one. */
  it('disposes the addon when the context is lost', () => {
    withAContext({})
    const dispose = vi.fn()
    const heard: (() => void)[] = []
    drawOnTheGpu(terminal() as never, () => ({
      dispose,
      onContextLoss: (then: () => void) => heard.push(then),
    }) as never)

    expect(heard).toHaveLength(1)
    heard[0]!()
    expect(dispose).toHaveBeenCalledOnce()
  })

  /* The case every test run and every headless box is in. Asked before the
     addon is built, because a null context does not throw — it fails later,
     somewhere else, and the first version of this hung a suite when it did. */
  it('builds no addon at all where there is no context to be had', () => {
    withAContext(null)
    const made = vi.fn()
    const pane = terminal()

    expect(drawOnTheGpu(pane as never, made as never)).toBe(false)
    expect(made).not.toHaveBeenCalled()
    expect(pane.loadAddon).not.toHaveBeenCalled()
  })
})
