import { useCallback } from 'react'

import { keptAsPicture, pastedPicture } from './pasting'
import { clipboardPng } from './terminalClipboard'

/*
 * What the composer takes in besides typing: a pasted picture, and files
 * dropped on the chat.
 */
export function useTaking(chat: { paste: (file: Blob) => void; attach: (paths: readonly string[]) => void }): {
  pasted: (event: React.ClipboardEvent<HTMLTextAreaElement>) => void
  dropped: (dropped: { paths: readonly string[]; files: readonly File[] }) => void
} {
  /* A picture pasted into the composer is an attachment. WebKitGTK leaves it
     out of the paste event, so a paste with neither a picture nor text asks
     the system clipboard itself — which is where a screenshot tool put it. */
  const pasted = (event: React.ClipboardEvent<HTMLTextAreaElement>): void => {
    const picture = pastedPicture(event.clipboardData)
    if (picture) {
      event.preventDefault()
      return chat.paste(picture)
    }
    if (event.clipboardData?.getData('text/plain')) return
    event.preventDefault()
    void clipboardPng().then((png) => png && chat.paste(png))
  }
  /* Pictures dropped in are kept like a pasted one, wherever they came from,
     when the chat can keep them; other files are attached by path, and only
     from inside the project. */
  const { paste: keep, attach } = chat
  const dropped = useCallback(
    ({ paths, files }: { paths: readonly string[]; files: readonly File[] }) => {
      const pictures = files.filter(keptAsPicture)
      for (const picture of pictures) keep(picture)
      /* A dropped file carries its name and not its folder, so each kept
         picture accounts for one path of that name, not every one. */
      const named = pictures.map((picture) => picture.name)
      const rest = paths.filter((path) => {
        const at = named.indexOf(path.slice(path.lastIndexOf('/') + 1))
        if (at < 0) return true
        named.splice(at, 1)
        return false
      })
      if (rest.length > 0) attach(rest)
    },
    [keep, attach],
  )
  return { pasted, dropped }
}
