import { useEffect, useRef, useState } from 'react'

/*
 * A mermaid diagram, drawn only if one is on screen.
 *
 * Mermaid is two megabytes and most files have none, so it is imported at the
 * moment a fenced `mermaid` block is actually rendered rather than bundled
 * into the shell. A diagram that fails to parse falls back to its own source:
 * the text is what the person wrote, and showing it beats an empty frame.
 */

export function Diagram({ source }: { source: string }): React.JSX.Element {
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
