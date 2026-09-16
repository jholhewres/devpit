import { useEffect, useRef } from 'react'

/*
 * The text editor every Capability that edits text uses.
 *
 * CodeMirror 6, loaded through `import()` like the other editors: a person
 * who never opens one of these should not pay for it at launch.
 *
 * One editor rather than one per Capability. A textarea would have done for
 * the first of them and would have been replaced by the second, and two
 * editors in one window is two sets of keys that disagree about what Home
 * does.
 */

export function TextEditor({
  value,
  onChange,
  language,
  label,
}: {
  /** Read once per generation: the editor owns the text after that, and
   *  writing every keystroke back into it would fight the cursor. */
  value: string
  onChange: (text: string) => void
  /** `markdown`, or plain text when absent. */
  language?: 'markdown'
  label: string
}): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  /* The callback the editor was mounted with goes stale; this does not. */
  const latest = useRef(onChange)
  latest.current = onChange

  useEffect(() => {
    const where = box.current
    if (!where) return undefined
    let view: { destroy: () => void } | null = null
    let dropped = false

    void (async () => {
      const [{ EditorView, keymap, lineNumbers }, { EditorState }, { defaultKeymap, history, historyKeymap }] =
        await Promise.all([
          import('@codemirror/view'),
          import('@codemirror/state'),
          import('@codemirror/commands'),
        ])
      const extensions = [
        lineNumbers(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        EditorView.lineWrapping,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) latest.current(update.state.doc.toString())
        }),
      ]
      if (language === 'markdown') {
        const { markdown } = await import('@codemirror/lang-markdown')
        extensions.push(markdown())
      }
      if (dropped) return
      view = new EditorView({
        state: EditorState.create({ doc: value, extensions }),
        parent: where,
      })
      where.querySelector('.cm-content')?.setAttribute('aria-label', label)
    })()

    return () => {
      dropped = true
      view?.destroy()
    }
    // Mounted once per file and per read: `value` is the starting text, not a
    // controlled prop, and listing it here would rebuild the editor on every
    // keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [language, label])

  return <div className="ted" ref={box} />
}
