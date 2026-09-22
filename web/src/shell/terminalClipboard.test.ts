import { beforeEach, describe, expect, it, vi } from 'vitest'

const clip = vi.hoisted(() => ({ readText: vi.fn(), writeText: vi.fn(), readImage: vi.fn() }))
vi.mock('@tauri-apps/plugin-clipboard-manager', () => clip)
const chatPaste = vi.hoisted(() => vi.fn())
vi.mock('./live', () => ({
  ask: (call: () => Promise<unknown>) => call(),
  commands: { chatPaste },
}))

import type { Terminal } from '@xterm/xterm'
import { clipboardKey, clipboardLabels, copySelection, pasteClipboard } from './terminalClipboard'

const key = (key: string, mods: Partial<KeyboardEvent> = {}) => ({
  key,
  ctrlKey: false,
  shiftKey: false,
  metaKey: false,
  altKey: false,
  ...mods,
})

const terminal = (selection = '') => {
  const pasted = vi.fn()
  return { t: { getSelection: () => selection, paste: pasted } as unknown as Terminal, pasted }
}

describe('the keys that copy and paste', () => {
  it('are Ctrl+Shift+C and Ctrl+Shift+V off a Mac, leaving Ctrl+C to the program', () => {
    expect(clipboardKey(key('C', { ctrlKey: true, shiftKey: true }), false)).toBe('copy')
    expect(clipboardKey(key('V', { ctrlKey: true, shiftKey: true }), false)).toBe('paste')
    expect(clipboardKey(key('c', { ctrlKey: true }), false)).toBeNull()
    expect(clipboardKey(key('v', { ctrlKey: true }), false)).toBeNull()
  })

  it('take Shift+Insert and Ctrl+Insert as a terminal does', () => {
    expect(clipboardKey(key('Insert', { shiftKey: true }), false)).toBe('paste')
    expect(clipboardKey(key('Insert', { ctrlKey: true }), false)).toBe('copy')
  })

  it('are ⌘C and ⌘V on a Mac', () => {
    expect(clipboardKey(key('c', { metaKey: true }), true)).toBe('copy')
    expect(clipboardKey(key('v', { metaKey: true }), true)).toBe('paste')
    expect(clipboardKey(key('c', { ctrlKey: true }), true)).toBeNull()
  })
})

describe('what the menu prints', () => {
  it('is the keys that are listened for', () => {
    expect(clipboardLabels(true)).toEqual({ copy: '⌘C', paste: '⌘V' })
    expect(clipboardKey(key('c', { metaKey: true }), true)).toBe('copy')
    expect(clipboardLabels(false)).toEqual({ copy: 'Ctrl+Shift+C', paste: 'Ctrl+Shift+V' })
    expect(clipboardKey(key('V', { ctrlKey: true, shiftKey: true }), false)).toBe('paste')
  })
})

describe('copy and paste', () => {
  beforeEach(() => {
    for (const one of Object.values(clip)) one.mockReset()
    chatPaste.mockReset()
  })

  it('copies the selection, and nothing when there is none', async () => {
    expect(await copySelection(terminal('ls -la').t)).toBe(true)
    expect(clip.writeText).toHaveBeenCalledWith('ls -la')
    expect(await copySelection(terminal('').t)).toBe(false)
    expect(clip.writeText).toHaveBeenCalledTimes(1)
  })

  it('pastes text as text', async () => {
    clip.readText.mockResolvedValue('echo hi')
    const { t, pasted } = terminal()
    expect(await pasteClipboard(t, 'prj')).toBeNull()
    expect(pasted).toHaveBeenCalledWith('echo hi')
    expect(clip.readImage).not.toHaveBeenCalled()
  })

  it('says why when the clipboard holds nothing it can take', async () => {
    clip.readText.mockRejectedValue(new Error('no text'))
    clip.readImage.mockRejectedValue(new Error('no image on the clipboard'))
    const { t, pasted } = terminal()
    expect(await pasteClipboard(t, 'prj')).toBe('no image on the clipboard')
    expect(pasted).not.toHaveBeenCalled()
  })
})
