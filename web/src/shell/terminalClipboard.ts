import { readImage, readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import type { Terminal } from '@xterm/xterm'

import { ask, commands } from './live'
import { reason } from './reason'

/*
 * Copy and paste in a terminal.
 *
 * Through the clipboard plugin, not the webview's: on WebKitGTK a terminal's
 * hidden textarea never sees a copy it can answer, `navigator.clipboard` is
 * refused, and neither can read a picture. With the plugin, Copy takes the
 * selection, Paste takes text — or a picture, kept where the chat keeps one
 * and pasted as its path, which Claude Code and Codex read as the image.
 */

export type ClipboardAct = 'copy' | 'paste'

const mac = (): boolean => typeof navigator !== 'undefined' && /Mac/.test(navigator.platform)

/** The terminal's copy and paste keys: ⇧⌃C/⇧⌃V and ⌃/⇧Insert off a Mac, ⌘C/⌘V
 *  on one. Plain ⌃C and ⌃V stay the program's: interrupt, and Claude Code's
 *  own paste-an-image. */
export function clipboardKey(
  event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'metaKey' | 'altKey'>,
  onMac = mac(),
): ClipboardAct | null {
  if (event.altKey) return null
  const key = event.key.toLowerCase()
  if (onMac) {
    if (!event.metaKey || event.ctrlKey) return null
    return key === 'c' ? 'copy' : key === 'v' ? 'paste' : null
  }
  if (key === 'insert') return event.shiftKey ? 'paste' : event.ctrlKey ? 'copy' : null
  if (!event.ctrlKey || !event.shiftKey || event.metaKey) return null
  return key === 'c' ? 'copy' : key === 'v' ? 'paste' : null
}

/** What the menu prints beside Copy and Paste — the keys `clipboardKey` takes.
 *  Kept beside the listener, as `tileKeys.ts` keeps its own, because these
 *  belong to the terminal and not to the window's shortcut map. */
export function clipboardLabels(onMac = mac()): Record<ClipboardAct, string> {
  return onMac ? { copy: '⌘C', paste: '⌘V' } : { copy: 'Ctrl+Shift+C', paste: 'Ctrl+Shift+V' }
}

/** Copies the selection, and says whether there was one to copy. */
export async function copySelection(terminal: Terminal): Promise<boolean> {
  const selected = terminal.getSelection()
  if (!selected) return false
  await writeText(selected)
  return true
}

/** Pastes what the clipboard holds. Answers with why it could not, or null. */
export async function pasteClipboard(terminal: Terminal, projectId: string): Promise<string | null> {
  const text = await readText().catch(() => '')
  if (text) {
    terminal.paste(text)
    return null
  }
  try {
    const picture = await readImage()
    const { width, height } = await picture.size()
    const data = pngOf(await picture.rgba(), width, height)
    const answer = await ask(() => commands.chatPaste(projectId, 'image/png', data))
    if (!answer.data) return answer.error ?? 'the picture could not be kept'
    terminal.paste(answer.data.path)
    return null
  } catch (thrown) {
    return reason(thrown, 'the clipboard holds nothing a terminal can take')
  }
}

/** Raw RGBA as a base64 PNG, drawn through a canvas. */
function pngOf(rgba: Uint8Array, width: number, height: number): string {
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const context = canvas.getContext('2d')
  if (!context) throw new Error('there is no canvas to encode the picture with')
  context.putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0)
  const url = canvas.toDataURL('image/png')
  return url.slice(url.indexOf(',') + 1)
}
