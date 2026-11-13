import { pathsIn } from './terminalLinks'

/* Code in a line of prose. A file named in backticks — how an answer usually
   points at one — opens like a link. */
export function Tick({ text, opens }: { text: string; opens: ((path: string) => void) | null }): React.JSX.Element {
  const named = pathsIn(text)
  const whole = named.length === 1 && named[0].start === 0 && named[0].end === text.length
  if (!whole || !opens) return <code className="md__tick">{text}</code>
  return (
    <button className="md__a md__path" onClick={() => opens(named[0].path)}>
      <code className="md__tick">{text}</code>
    </button>
  )
}

/* Running text with the files it names as links: `src/a.ts:12` in an answer
   or a tool's output opens beside the chat. */
export function Pathed({ text, opens }: { text: string; opens: ((path: string) => void) | null }): React.JSX.Element {
  const named = opens ? pathsIn(text) : []
  if (!opens || named.length === 0) return <>{text}</>
  const out: React.ReactNode[] = []
  let at = 0
  for (const one of named) {
    out.push(text.slice(at, one.start))
    out.push(
      <button key={one.start} className="md__a md__path" onClick={() => opens(one.path)}>
        {text.slice(one.start, one.end)}
      </button>,
    )
    at = one.end
  }
  out.push(text.slice(at))
  return <>{out}</>
}
