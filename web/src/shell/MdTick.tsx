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
