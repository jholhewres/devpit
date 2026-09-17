import { useEffect, useRef } from 'react'

/*
 * A note, edited the way it reads.
 *
 * Milkdown's Crepe: a block editor with a slash menu, drag handles and
 * formatting in place — the shape people mean when they say Notion — over
 * ProseMirror.
 *
 * Markdown is its own format, in and out, and that is why it was picked over
 * the editors that look closer to Notion still. A note here is a `.md` file in
 * the project's folder, which is the whole promise of the Capability: a JSON
 * document of blocks converted to Markdown on every save would lose something
 * every time somebody edited a file another tool wrote.
 *
 * Loaded through `import()`, like every other editor in this window.
 */

export function NoteEditor({
  value,
  onChange,
  dark,
}: {
  /** Read once per mount: the editor owns the document after that. */
  value: string
  onChange: (markdown: string) => void
  dark: boolean
}): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  /* The callback the editor was built with goes stale; this does not. */
  const latest = useRef(onChange)
  latest.current = onChange

  useEffect(() => {
    const where = box.current
    if (!where) return undefined
    let crepe: { destroy: () => Promise<unknown>; getMarkdown: () => string } | null = null
    let dropped = false

    void (async () => {
      const [{ Crepe }, { listener, listenerCtx }] = await Promise.all([
        import('@milkdown/crepe'),
        import('@milkdown/kit/plugin/listener'),
      ])
      await import(dark ? '@milkdown/crepe/theme/frame-dark.css' : '@milkdown/crepe/theme/frame.css')
      await import('@milkdown/crepe/theme/common/style.css')

      const made = new Crepe({ root: where, defaultValue: value })
      made.editor.use(listener)
      made.editor.config((ctx) => {
        ctx.get(listenerCtx).markdownUpdated((_ctx, markdown) => latest.current(markdown))
      })
      await made.create()
      if (dropped) {
        void made.destroy()
        return
      }
      crepe = made
    })()

    return () => {
      dropped = true
      void crepe?.destroy()
    }
    // Built once per file and per read: `value` is the starting document, not
    // a controlled prop, and listing it here would rebuild the editor — and
    // move the cursor — on every keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dark])

  return <div className="note__ed" ref={box} />
}
