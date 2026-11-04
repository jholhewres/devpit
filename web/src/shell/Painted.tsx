import { memo, useMemo, useRef } from 'react'

import { grownTokens, type Painting, type Token } from './highlight'
import type { Language } from './languages'

/*
 * Source text as coloured spans.
 *
 * Nodes and never a string of HTML, which is the same promise `Markdown.tsx`
 * makes and for the same reason: a file containing `<script>` arrives here as
 * a token whose text is `<script>` and is drawn as text. Structural rather
 * than a sanitiser somebody has to keep ahead of.
 *
 * Its own component because two surfaces paint the same thing — the editor,
 * behind its textarea, and a fenced block in a Markdown preview.
 */
export function Painted({
  text,
  language,
}: {
  text: string
  language: Language | null
}): React.JSX.Element {
  /* A block being written is coloured from its last opened line on, and the
     runs above it, the same tokens, are not drawn again. */
  const was = useRef<Painting | null>(null)
  const painted = useMemo(() => (was.current = grownTokens(was.current, text, language)).tokens, [text, language])
  const runs: (readonly Token[])[] = []
  for (let at = 0; at < painted.length; at += RUN) runs.push(painted.slice(at, at + RUN))
  return (
    <>
      {runs.map((run, at) => (
        <Run key={at} run={run} />
      ))}
    </>
  )
}

/* Tokens per run drawn as one. */
const RUN = 200

const Run = memo(
  function Run({ run }: { run: readonly Token[] }): React.JSX.Element {
    return (
      <>
        {run.map((token, at) =>
          token.kind === 'plain' ? (
            token.text
          ) : (
            <span className={`tok tok--${token.kind}`} key={at}>
              {token.text}
            </span>
          ),
        )}
      </>
    )
  },
  (was, now) => was.run.length === now.run.length && was.run.every((token, at) => token === now.run[at]),
)
