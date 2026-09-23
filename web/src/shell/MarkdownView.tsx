import { createContext, useCallback, useContext, useMemo } from 'react'

import { Diagram } from './Diagram'
import { ofName } from './languages'
import { MdTable } from './MdTable'
import { Painted } from './Painted'
import { blocks, external, resolved, spans, type Block, type Span } from './markdown'
import { ask, commands } from './live'
import { cut, locate, styleOf, type Chunk } from './veil'
import { useShellPick } from './shellStore'

/*
 * Markdown, drawn from parsed blocks.
 *
 * Never from a string of HTML: nothing in `markdown.ts` produces markup, so
 * raw HTML in the source arrives here as text and is drawn as text. That is
 * the guarantee, and it is structural rather than a sanitiser someone has to
 * keep ahead of.
 */

/* Where the fade is up to. One cursor for the whole pass, walked forward in
   document order, so a word that appears twice fades on the right one. It is
   context rather than a prop because every node would otherwise have to carry
   it down to the one that draws text. */
interface Fading {
  readonly source: string
  readonly chunks: readonly Chunk[]
  readonly now: number
  cursor: number
}

const Veil = createContext<Fading | null>(null)

/* Context and not a prop for the same reason as the fade: every node between
   the document and the one link would otherwise have to carry it down. */
const Opening = createContext<((path: string) => void) | null>(null)

export function Markdown({
  source,
  path,
  chunks,
  now,
  opens,
}: {
  source: string
  path?: string
  chunks?: readonly Chunk[]
  now?: number
  /* What a link to a file beside this document means. Absent, it is a file in
     the project, which is what every caller but one wants. The exception is a
     `SKILL.md`, which lives outside every project — its neighbours cannot be
     opened as tabs, and following one as if they could ends in a refusal the
     reader cannot act on. */
  opens?: (path: string) => void
}): React.JSX.Element {
  const fading = chunks?.length ? { source, chunks, now: now ?? Date.now(), cursor: 0 } : null
  /* Once here, not in every link: a long answer has hundreds of them. */
  const show = useShellPick((shell) => shell.show)
  const opener = useCallback(
    (here: string) => (opens ? opens(here) : show('file', { id: `file:${here}`, path: here })),
    [opens, show],
  )
  const parsed = useMemo(() => blocks(source), [source])

  return (
    <Veil.Provider value={fading}>
      <div className="md">
        <Opening.Provider value={opener}>
          {parsed.map((block, at) => (
            <Piece key={at} block={block} path={path ?? ''} />
          ))}
        </Opening.Provider>
      </div>
    </Veil.Provider>
  )
}

/* A run of text, cut where the fading ranges begin and end. Off the fading
   path this is one string and one node, exactly as before. */
function Text({ text }: { text: string }): React.JSX.Element {
  const fading = useContext(Veil)
  if (!fading) return <>{text}</>

  const at = locate(fading.source, text, fading.cursor)
  if (at < 0) return <>{text}</>
  fading.cursor = at + text.length

  return (
    <>
      {cut(text, at, fading.chunks).map((slice, index) =>
        slice.chunk ? (
          <span className="veil" key={index} style={styleOf(slice.chunk, fading.now, fading.chunks.length)}>
            {slice.text}
          </span>
        ) : (
          <span key={index}>{slice.text}</span>
        ),
      )}
    </>
  )
}

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
      return <code className="md__tick">{span.text}</code>
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
    case 'link':
      return (
        <button
          className="md__a"
          onClick={() => {
            /* A link that leaves the machine opens in the system browser; the
               window is not a browser and must not become one. */
            if (external(span.href)) return void ask(() => commands.pathOpen(span.href))
            const here = resolved(path, span.href)
            opens?.(here)
          }}
        >
          {span.text}
        </button>
      )
    default:
      return <Text text={span.text} />
  }
}
