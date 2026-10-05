import { createContext, memo, useCallback, useContext, useMemo } from 'react'

import { Diagram } from './Diagram'
import { ofName } from './languages'
import { MdTable } from './MdTable'
import { Painted } from './Painted'
import { blocks, external, linkTarget, resolved, spans, type Block, type Span } from './markdown'
import { Tick } from './MdTick'
import { ask, commands } from './live'
import { went } from './problems'
import { useShellPick } from './shellStore'

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
}: {
  source: string
  path?: string
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
  const parsed = useMemo(() => blocks(source), [source])

  return (
    <div className="md">
      <Opening.Provider value={opener}>
        {parsed.map((block, at) => (
          <Kept key={at} block={block} path={path ?? ''} />
        ))}
      </Opening.Provider>
    </div>
  )
}

/* A block drawn again only when it changed: an answer being written changes
   its last block, and the ones above it hold still. */
const Kept = memo(
  (props: { block: Block; path: string }) => <Piece {...props} />,
  (was, now) => was.path === now.path && JSON.stringify(was.block) === JSON.stringify(now.block),
)

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
        <pre className="md__code">
          <code>
            <Painted text={block.text} language={ofName(block.language)} />
          </code>
        </pre>
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
        <img className="md__img" src={span.src} alt={span.alt} />
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
      return <>{span.text}</>
  }
}
