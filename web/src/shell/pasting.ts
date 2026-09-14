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

