/*
 * What a paste into the composer holds.
 *
 * Its own file so the rule is tested without mounting a chat: a picture is an
 * attachment, and everything else is typing that must reach the field.
 */

/* The first picture on a clipboard, or nothing when it holds only text. */
export function pastedPicture(data: DataTransfer | null): File | null {
  for (const item of Array.from(data?.items ?? [])) {
    if (item.kind === 'file' && item.type.startsWith('image/')) return item.getAsFile()
  }
  return null
}

/* A paste handler that takes a picture as an attachment and lets text through
   to the field, where it is typing. */
export const picturesTo =
  (paste: (file: File) => void) =>
  (event: { clipboardData: DataTransfer | null; preventDefault: () => void }): void => {
    const picture = pastedPicture(event.clipboardData)
    if (!picture) return
    event.preventDefault()
    paste(picture)
  }


/* The paths a drop from the file manager names: WebKitGTK hands the files
   over without their paths, and says where they are in `text/uri-list`. */
export function droppedPaths(data: DataTransfer | null): string[] {
  const list = data?.getData('text/uri-list') ?? ''
  return list
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith('file://'))
    .flatMap((line) => {
      /* One entry that does not parse costs that entry, not the drop; and a
         file on another host is not at that path on this one. */
      try {
        const url = new URL(line)
        if (url.host !== '' && url.host !== 'localhost') return []
        return [decodeURIComponent(url.pathname)]
      } catch {
        return []
      }
    })
}

/* The pictures the chat keeps as a paste, as `chat_paste` takes them: other
   pictures, and bigger ones, are attached by path like any file. */
const KEPT_TYPES = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp'])
export const KEPT_BYTES = 8 * 1024 * 1024
export function keptAsPicture(file: Blob): boolean {
  return KEPT_TYPES.has(file.type) && file.size <= KEPT_BYTES
}
