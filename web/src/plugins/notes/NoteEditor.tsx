import { useEffect, useRef, useState } from 'react'
import type { CmdKey } from '@milkdown/kit/core'
import type { Editor } from '@milkdown/kit/core'
import {
  toggleEmphasisCommand,
  toggleInlineCodeCommand,
  toggleStrongCommand,
  turnIntoTextCommand,
  wrapInBlockquoteCommand,
  wrapInBulletListCommand,
  wrapInHeadingCommand,
  wrapInOrderedListCommand,
  insertHrCommand,
} from '@milkdown/kit/preset/commonmark'
import { callCommand } from '@milkdown/kit/utils'

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

/** One button on the bar: what it says, what it does, and how it reads out. */
interface Action {
  readonly label: string
  readonly title: string
  readonly run: (editor: Editor) => void
  /** A rule, not a button: drawn as a gap in the row. */
  readonly gap?: boolean
}

/* `callCommand` takes the key and an optional payload, which is why headings
   are three entries rather than one with an argument the bar would have to
   hold. */
const call = <T,>(key: CmdKey<T>, payload?: T) => (editor: Editor): void => {
  editor.action(callCommand(key, payload))
}

const ACTIONS: readonly Action[] = [
  { label: 'H1', title: 'Heading 1', run: call(wrapInHeadingCommand.key, 1) },
  { label: 'H2', title: 'Heading 2', run: call(wrapInHeadingCommand.key, 2) },
  { label: 'H3', title: 'Heading 3', run: call(wrapInHeadingCommand.key, 3) },
  { label: 'Text', title: 'Plain paragraph', run: call(turnIntoTextCommand.key), gap: true },
  { label: 'B', title: 'Bold', run: call(toggleStrongCommand.key), gap: true },
  { label: 'I', title: 'Italic', run: call(toggleEmphasisCommand.key) },
  { label: '<>', title: 'Inline code', run: call(toggleInlineCodeCommand.key) },
  { label: '•', title: 'Bulleted list', run: call(wrapInBulletListCommand.key), gap: true },
  { label: '1.', title: 'Numbered list', run: call(wrapInOrderedListCommand.key) },
  { label: '❝', title: 'Quote', run: call(wrapInBlockquoteCommand.key) },
  { label: '—', title: 'Divider', run: call(insertHrCommand.key) },
]

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
  /* The bar acts on this, and stays disabled until the editor is up: a
     command called before `create()` lands reaches no document. */
  const [editor, setEditor] = useState<Editor | null>(null)

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
      setEditor(made.editor)
    })()

    return () => {
      dropped = true
      setEditor(null)
      void crepe?.destroy()
    }
    // Built once per file and per read: `value` is the starting document, not
    // a controlled prop, and listing it here would rebuild the editor — and
    // move the cursor — on every keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dark])

  return (
    <>
      {/* Crepe's own menus arrive on a selection or a slash, which is a menu
          you have to know is there. This bar is the one that is simply there:
          the same commands, said out loud, above the page. */}
      <div className="note__bar" role="toolbar" aria-label="Formatting">
        {ACTIONS.map((action) => (
          <button
            key={action.label}
            className="note__act"
            data-gap={action.gap ? 'true' : undefined}
            type="button"
            title={action.title}
            aria-label={action.title}
            disabled={editor === null}
            /* The selection is what every one of these acts on, and a click
               that moves focus is a click that threw it away first. */
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => editor && action.run(editor)}
          >
            {action.label}
          </button>
        ))}
      </div>
      <div className="note__ed" ref={box} />
    </>
  )
}
