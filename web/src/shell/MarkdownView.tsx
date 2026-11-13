import { createContext, memo, useCallback, useContext, useMemo, useRef } from 'react'

import { CopyButton } from './CopyButton'
import { Diagram } from './Diagram'
import { ofName } from './languages'
import { MdTable } from './MdTable'
import { Painted } from './Painted'
import { external, grown, linkTarget, mended, resolved, spans, type Block, type Parsed, type Span } from './markdown'
import { Pathed, Tick } from './MdTick'
import { ask, commands } from './live'
import { went } from './problems'
import { useShellPick } from './shellStore'
import { Zoomable } from './Zoom'

/*
 * Markdown, drawn from parsed blocks.
 *
 * Never from a string of HTML: nothing in `markdown.ts` produces markup, so
 * raw HTML in the source arrives here as text and is drawn as text. That is
 * the guarantee, and it is structural rather than a sanitiser someone has to
 * keep ahead of.
 */

/* Context and not a prop: every node between the document and the one link
   would otherwise have to carry it down. */
const Opening = createContext<((path: string) => void) | null>(null)

export function Markdown({
  source,
  path,
  opens,
  live = false,
}: {
  source: string
  path?: string
  /** Still being written: its last block closes what it left open. */
  live?: boolean
  /* What a link to a file beside this document means. Absent, it is a file in
     the project, which is what every caller but one wants. The exception is a
     `SKILL.md`, which lives outside every project — its neighbours cannot be
     opened as tabs, and following one as if they could ends in a refusal the
     reader cannot act on. */
  opens?: (path: string) => void
}): React.JSX.Element {
  /* Once here, not in every link: a long answer has hundreds of them. */
  const show = useShellPick((shell) => shell.show)
  const opener = useCallback(
    (here: string) => (opens ? opens(here) : show('file', { id: `file:${here}`, path: here })),
    [opens, show],
  )
  /* Text that only grew is read from its last open block on, not from the top. */
  const was = useRef<Parsed | null>(null)
  const parsed = useMemo(() => (was.current = grown(was.current, source)).blocks, [source])

  return (
    <div className="md">
      <Opening.Provider value={opener}>
        {parsed.map((block, at) => (
          <Kept key={at} block={live && at === parsed.length - 1 ? mended(block) : block} path={path ?? ''} />
        ))}
      </Opening.Provider>
    </div>
  )
}

/* A block drawn again only when it changed: an answer being written changes
   its last block, and the ones above it are the same objects. */
const Kept = memo((props: { block: Block; path: string }) => <Piece {...props} />)

function Piece({ block, path }: { block: Block; path: string }): React.JSX.Element | null {
  switch (block.kind) {
    case 'heading': {
      const Tag = `h${Math.min(block.level, 6)}` as 'h1'
      return <Tag className="md__h">{<Inline text={block.text} path={path} />}</Tag>
    }
    case 'paragraph':
      return (
        <p className="md__p">
          <Inline text={block.text} path={path} />
        </p>
      )
    case 'code':
      return block.language === 'mermaid' ? (
        <Diagram source={block.text} />
      ) : (
        <div className="md__block">
          <pre className="md__code">
            <code>
              <Painted text={block.text} language={ofName(block.language)} />
            </code>
          </pre>
          {/* The block's text as parsed: what is between the fences, never them. */}
          <CopyButton className="md__copy" text={block.text} label="Copy code" />
        </div>
      )
    case 'list': {
      const Tag = block.ordered ? 'ol' : 'ul'
      return (
        <Tag className="md__list">
          {block.items.map((item, at) => (
            <li key={at}>
              <Inline text={item} path={path} />
            </li>
          ))}
        </Tag>
      )
    }
    case 'quote':
      return (
        <blockquote className="md__quote">
          <Inline text={block.text} path={path} />
        </blockquote>
      )
    case 'rule':
      return <hr className="md__rule" />
    case 'table':
      return <MdTable block={block} cell={(text) => <Inline text={text} path={path} />} />
    default:
      return null
  }
}

function Inline({ text, path }: { text: string; path: string }): React.JSX.Element {
  return (
    <>
      {spans(text).map((span, at) => (
        <Bit key={at} span={span} path={path} />
      ))}
    </>
  )
}

function Bit({ span, path }: { span: Span; path: string }): React.JSX.Element {
  const opens = useContext(Opening)

  switch (span.kind) {
    case 'code':
      return <Tick text={span.text} opens={opens} />
    case 'strong':
      return <strong>{span.text}</strong>
    case 'em':
      return <em>{span.text}</em>
    case 'image':
      /* Resolved against the file's own folder. Nothing loads it yet unless it
         is a URL — a relative image needs a read through the backend, and this
         says so rather than drawing a broken frame. */
      return external(span.src) ? (
        <Zoomable src={span.src} name={span.alt || 'Image'} className="md__img" />
      ) : (
        <span className="md__tick">{resolved(path, span.src)}</span>
      )
    case 'link': {
      const target = linkTarget(span.href)
      if (target === 'text') return <span title={span.href}>{span.text}</span>
      /* The web opens in the system browser: the window is not a browser and must not become one. */
      const open = (): void =>
        target === 'web' ? void ask(() => commands.urlOpen(span.href)).then(went) : opens?.(resolved(path, span.href))
      return <button className="md__a" title={target === 'web' ? span.href : undefined} onClick={open}>{span.text}</button>
    }
    default:
      return <Pathed text={span.text} opens={opens} />
  }
}
