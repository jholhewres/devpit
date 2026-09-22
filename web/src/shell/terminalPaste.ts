import { ask, commands } from './live'
import { pastedPicture } from './pasting'
import { reason } from './reason'

/*
 * A picture pasted into a terminal.
 *
 * A terminal only takes text, so the picture is kept where the chat keeps
 * one and its path is pasted instead. Claude Code and Codex read a pasted
 * image path as the image itself — the same thing a native terminal gets from
 * dragging a file in. Text is left alone: xterm pastes it as it always did.
 *
 * Listened for on the way down, before xterm's own textarea sees the event,
 * because xterm would otherwise paste an empty string and swallow the rest.
 */

export function picturesAsPaths(
  projectId: string,
  paste: (path: string) => void,
  failed: (why: string | null) => void,
): (event: ClipboardEvent) => void {
  return (event) => {
    const picture = pastedPicture(event.clipboardData)
    if (!picture) return
    event.preventDefault()
    event.stopPropagation()
    void base64Of(picture)
      .then((data) => ask(() => commands.chatPaste(projectId, picture.type, data)))
      .then((answer) => {
        if (!answer.data) return failed(answer.error ?? 'the picture could not be kept')
        failed(null)
        paste(answer.data.path)
      })
      .catch((thrown: unknown) => failed(reason(thrown, 'the picture could not be kept')))
  }
}

function base64Of(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const url = typeof reader.result === 'string' ? reader.result : ''
      resolve(url.slice(url.indexOf(',') + 1))
    }
    reader.onerror = () => reject(reader.error ?? new Error('the picture could not be read'))
    reader.readAsDataURL(file)
  })
}
