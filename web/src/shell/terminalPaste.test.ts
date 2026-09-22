import { beforeEach, describe, expect, it, vi } from 'vitest'

const chatPaste = vi.fn()
vi.mock('./live', () => ({
  ask: (call: () => Promise<unknown>) => call(),
  commands: { chatPaste: (...args: unknown[]) => chatPaste(...args) },
}))

import { picturesAsPaths } from './terminalPaste'

function pasteOf(items: { kind: string; type: string; file?: File }[]) {
  const event = {
    clipboardData: {
      items: items.map((item) => ({ ...item, getAsFile: () => item.file ?? null })),
    } as unknown as DataTransfer,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  }
  return event as unknown as ClipboardEvent & typeof event
}

describe('a paste into a terminal', () => {
  beforeEach(() => chatPaste.mockReset())

  it('turns a picture into the path it was kept under', async () => {
    chatPaste.mockResolvedValue({ data: { path: '/home/me/.devpit/p/pasted/a.png' } })
    const paste = vi.fn()
    const failed = vi.fn()
    const event = pasteOf([
      { kind: 'file', type: 'image/png', file: new File(['png'], 'a.png', { type: 'image/png' }) },
    ])

    picturesAsPaths('prj_a', paste, failed)(event)
    await vi.waitFor(() => expect(paste).toHaveBeenCalled())

    expect(event.preventDefault).toHaveBeenCalled()
    expect(chatPaste).toHaveBeenCalledWith('prj_a', 'image/png', expect.any(String))
    expect(paste).toHaveBeenCalledWith('/home/me/.devpit/p/pasted/a.png')
    expect(failed).toHaveBeenLastCalledWith(null)
  })

  it('leaves text to xterm', () => {
    const paste = vi.fn()
    const event = pasteOf([{ kind: 'string', type: 'text/plain' }])
    picturesAsPaths('prj_a', paste, vi.fn())(event)
    expect(event.preventDefault).not.toHaveBeenCalled()
    expect(chatPaste).not.toHaveBeenCalled()
  })

  it('says why when the picture could not be kept', async () => {
    chatPaste.mockResolvedValue({ error: 'that picture is past the 8 MB a paste takes' })
    const paste = vi.fn()
    const failed = vi.fn()
    const event = pasteOf([
      { kind: 'file', type: 'image/png', file: new File(['png'], 'a.png', { type: 'image/png' }) },
    ])
    picturesAsPaths('prj_a', paste, failed)(event)
    await vi.waitFor(() => expect(failed).toHaveBeenCalled())
    expect(paste).not.toHaveBeenCalled()
    expect(failed).toHaveBeenCalledWith('that picture is past the 8 MB a paste takes')
  })
})
