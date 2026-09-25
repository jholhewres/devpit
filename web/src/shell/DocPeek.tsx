import { Markdown } from './MarkdownView'
import { useFile } from './useFile'
import { useShellPick } from './shellStore'

/*
 * A file an answer points at, read beside the chat rather than instead of it:
 * the conversation stays in view while the document is read. Expanding opens
 * it as a tab of its own, editable, where it can take the whole width.
 */
export function DocPeek({ path, onOpen, onClose }: { path: string; onOpen: (path: string) => void; onClose: () => void }): React.JSX.Element {
  const show = useShellPick((shell) => shell.show)
  const file = useFile(path)
  const name = path.split('/').pop() ?? path
  const text = file.file?.text ?? null
  const markdown = /\.(md|markdown|mdx)$/i.test(path)

  return (
    <aside className="peek" aria-label={`Preview of ${name}`}>
      <header className="peek__bar">
        <span className="peek__name" title={path}>
          {name}
        </span>
        <button
          className="sq26"
          title="Open as a tab"
          aria-label="Open as a tab"
          onClick={() => (show('file', { id: `file:${path}`, path, title: name }), onClose())}
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" /></svg>
        </button>
        <button className="sq26" title="Close" aria-label="Close preview" onClick={onClose}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </header>
      <div className="peek__body">
        {file.error ? (
          <p className="peek__note">{file.error}</p>
        ) : text === null ? (
          <p className="peek__note">{file.file?.notShown ?? 'Reading…'}</p>
        ) : markdown ? (
          <Markdown source={text} path={path} opens={onOpen} />
        ) : (
          <pre className="peek__text">{text}</pre>
        )}
      </div>
    </aside>
  )
}
