import { tokens } from './highlight'
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
  return (
    <>
      {tokens(text, language).map((token, at) =>
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
}
