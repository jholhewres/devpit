import { useEffect, useRef, useState } from 'react'

import { blocks, external, resolved, spans, type Block, type Span } from './markdown'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * Markdown, drawn from parsed blocks.
 *
 * Never from a string of HTML: nothing in `markdown.ts` produces markup, so
 * raw HTML in the source arrives here as text and is drawn as text. That is
 * the guarantee, and it is structural rather than a sanitiser someone has to
 * keep ahead of.
 */

export function Markdown({ source, path }: { source: string; path?: string }): React.JSX.Element {
  return (
    <div className="md">
      {blocks(source).map((block, at) => (
        <Piece key={at} block={block} path={path ?? ''} />
      ))}
    </div>
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
          <code>{block.text}</code>
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
      return <>{span.text}</>
  }
}

/* Mermaid is loaded when a diagram is actually on screen: it is two megabytes
   and most files have none. */
function Diagram({ source }: { source: string }): React.JSX.Element {
  const box = useRef<HTMLDivElement>(null)
  const [failed, setFailed] = useState<string | null>(null)

  useEffect(() => {
    let dropped = false
    void (async () => {
      try {
        const mermaid = (await import('mermaid')).default
        mermaid.initialize({ startOnLoad: false, theme: 'base', securityLevel: 'strict' })
        const { svg } = await mermaid.render(`d${Math.random().toString(36).slice(2)}`, source)
        if (!dropped && box.current) box.current.innerHTML = svg
      } catch (thrown) {
        if (!dropped) setFailed((thrown as Error).message)
      }
    })()
    return () => {
      dropped = true
    }
  }, [source])

  if (failed) {
    return (
      <pre className="md__code">
        <code>{`${failed}\n\n${source}`}</code>
      </pre>
    )
  }
  return <div className="md__diagram" ref={box} />
}
