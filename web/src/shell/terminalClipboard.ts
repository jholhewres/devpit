import { readImage, readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import type { Terminal } from '@xterm/xterm'

import { onMacNow, type Press } from './keys'
import { ask, commands } from './live'
import { KEPT_BYTES } from './pasting'
import { reason } from './reason'
import { matches } from './shortcuts'

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

/** Copy or paste, by the keys the registry gives them. */
export function clipboardKey(event: Press, onMac = onMacNow()): ClipboardAct | null {
  if (matches('copy', event, onMac)) return 'copy'
  if (matches('paste', event, onMac)) return 'paste'
  return null
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
    if (width * height > MAX_PIXELS) return 'the picture on the clipboard is too big to paste'
    const data = pngOf(await picture.rgba(), width, height)
    const answer = await ask(() => commands.chatPaste(projectId, 'image/png', data))
    if (!answer.data) return answer.error ?? 'the picture could not be kept'
    terminal.paste(answer.data.path)
    return null
  } catch (thrown) {
    return reason(thrown, 'the clipboard holds nothing a terminal can take')
  }
}

/** The picture on the system clipboard as a PNG, or null when it holds none.
 *  Read through the app: a paste event in WebKitGTK carries no picture. */
export async function clipboardPng(): Promise<Blob | null> {
  try {
    const picture = await readImage()
    const { width, height } = await picture.size()
    // Refused before its pixels cross over: an 8K screen is 130 MB of them.
    if (width * height > MAX_PIXELS) return null
    const canvas = canvasOf(await picture.rgba(), width, height)
    const png = await new Promise<Blob | null>((done) => canvas.toBlob(done, 'image/png'))
    return png && png.size <= KEPT_BYTES ? png : null
  } catch {
    return null
  }
}

/** The most pixels a clipboard picture may have to be taken: a 5K screen. */
const MAX_PIXELS = 40_000_000

/** Raw RGBA drawn on a canvas, to encode from. */
function canvasOf(rgba: Uint8Array, width: number, height: number): HTMLCanvasElement {
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const context = canvas.getContext('2d')
  if (!context) throw new Error('there is no canvas to encode the picture with')
  context.putImageData(new ImageData(new Uint8ClampedArray(rgba), width, height), 0, 0)
  return canvas
}

/** Raw RGBA as a base64 PNG. */
function pngOf(rgba: Uint8Array, width: number, height: number): string {
  const url = canvasOf(rgba, width, height).toDataURL('image/png')
  return url.slice(url.indexOf(',') + 1)
}
