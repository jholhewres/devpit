import { visible } from './diff'

/*
 * One diff line's text, with its whitespace made visible when asked.
 *
 * Apart from the pane so the pane stays a layout: this is the only part of it
 * that knows what a space looks like.
 */

export function DiffText({ text, spaces }: { text: string; spaces: boolean }): React.JSX.Element {
  if (!text) return <> </>
  if (!spaces) return <>{text}</>
  return (
    <>
      {visible(text).map((piece, at) =>
        piece.space ? (
          <span className="diff__ws" key={at}>
            {piece.text}
          </span>
        ) : (
          piece.text
        ),
      )}
    </>
  )
}
