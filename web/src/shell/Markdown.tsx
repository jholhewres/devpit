import { createContext, useContext } from 'react'

import { Diagram } from './Diagram'
import { ofName } from './languages'
import { Painted } from './Painted'
import { blocks, external, resolved, spans, type Block, type Span } from './markdown'
import { ask, commands } from './live'
import { cut, locate, styleOf, type Chunk } from './veil'
import { useShell } from './useShell'

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

export function Markdown({
  source,
  path,
  chunks,
  now,
}: {
  source: string
  path?: string
  chunks?: readonly Chunk[]
  now?: number
}): React.JSX.Element {
  const fading = chunks?.length ? { source, chunks, now: now ?? Date.now(), cursor: 0 } : null

  return (
    <Veil.Provider value={fading}>
      <div className="md">
        {blocks(source).map((block, at) => (
          <Piece key={at} block={block} path={path ?? ''} />
        ))}
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
  const { show } = useShell()

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
            if (external(span.href)) void ask(() => commands.pathOpen(span.href))
            else show('file', { id: `file:${resolved(path, span.href)}`, path: resolved(path, span.href) })
          }}
        >
          {span.text}
        </button>
      )
    default:
      return <Text text={span.text} />
  }
}
