import { describe, expect, it, vi } from 'vitest'

/* The addon stood in: a real one needs a GPU jsdom does not have. */
const disposed = vi.fn()
vi.mock('@xterm/addon-webgl', () => ({
  WebglAddon: class {
    onContextLoss = vi.fn()
    dispose = disposed
  },
}))

import { gpuWhileShown } from './terminalAddons'

describe('the GPU renderer of a pane', () => {
  it('is given back when the pane leaves the screen and taken again when it returns', () => {
    const original = HTMLCanvasElement.prototype.getContext
    HTMLCanvasElement.prototype.getContext = vi.fn(() => ({})) as never
    const pane = { loadAddon: vi.fn(), refresh: vi.fn(), rows: 24 }
    const gpu = gpuWhileShown(pane as never, () => {})

    gpu.show()
    gpu.show()
    expect(pane.loadAddon).toHaveBeenCalledOnce()

    gpu.hide()
    expect(disposed).toHaveBeenCalledOnce()
    gpu.hide()
    expect(disposed).toHaveBeenCalledOnce()

    gpu.show()
    expect(pane.loadAddon).toHaveBeenCalledTimes(2)
    HTMLCanvasElement.prototype.getContext = original
  })
})
