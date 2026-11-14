import { keysLabel } from './shortcuts'
import { setViewPref, SIZES, useViewPrefs, type ViewPref, type ViewPrefs } from './viewPrefs'

/* Rows of Settings kept in `viewPrefs`: the chat's keys in Sessions, the
   sizes, contrast, maths and diffs in Appearance. */

function Pick<K extends ViewPref>({
  field,
  title,
  said,
  choices,
}: {
  field: K
  title: string
  said: React.ReactNode
  choices: readonly (readonly [ViewPrefs[K], string])[]
}): React.JSX.Element {
  const value = useViewPrefs()[field]
  return (
    <div className="pref">
      <span className="pref__body">
        <span className="pref__t">{title}</span>
        <span className="pref__d">{said}</span>
      </span>
      <div className="seg seg--tight" role="radiogroup" aria-label={title}>
        {choices.map(([one, label]) => (
          <button key={String(one)} className="segb" role="radio" aria-checked={value === one} onClick={() => setViewPref(field, one)}>
            {label}
          </button>
        ))}
      </div>
    </div>
  )
}

const SIZE_LABELS: Record<(typeof SIZES)[number], string> = { small: 'Small', default: 'Default', large: 'Large', larger: 'Larger' }
const SIZE_CHOICES = SIZES.map((one) => [one, SIZE_LABELS[one]] as const)

export function ChatKeySettings(): React.JSX.Element {
  const other = keysLabel('steerNow')
  return (
    <>
      <Pick
        field="sendKey"
        title="Send with"
        said={`Enter sends and Shift+Enter starts a new line, or Enter starts a new line and ${other} sends.`}
        choices={[
          ['enter', 'Enter'],
          ['modEnter', other],
        ]}
      />
      <Pick
        field="followUp"
        title="While a turn runs"
        said={`What a message sent during a turn does: waits for the turn to end, or goes into it now. ${other} does the other, while Enter sends.`}
        choices={[
          ['queue', 'Queue'],
          ['steer', 'Steer'],
        ]}
      />
    </>
  )
}

export function LookSettings(): React.JSX.Element {
  return (
    <>
      <div className="acc__sub">Text</div>
      <Pick field="textSize" title="Text size" said="The conversation's text and its controls." choices={SIZE_CHOICES} />
      <Pick field="codeSize" title="Code size" said="Code blocks and code in a line, and every diff." choices={SIZE_CHOICES} />
      <Pick
        field="contrast"
        title="Contrast"
        said="More draws secondary text and borders closer to the main text."
        choices={[
          ['standard', 'Standard'],
          ['more', 'More'],
        ]}
      />
      <Pick
        field="latex"
        title="LaTeX maths"
        said={<>Draws <code>$…$</code> and <code>$$…$$</code> in answers and Markdown files as formulas. Off, a dollar sign is just a dollar sign.</>}
        choices={[
          [false, 'Off'],
          [true, 'On'],
        ]}
      />

      <div className="acc__sub">Diffs</div>
      <Pick
        field="diffLayout"
        title="Diff layout"
        said="How a diff opens. Each view keeps its own toggle; by default a file's diff opens side by side and a commit's stacked."
        choices={[
          ['each', 'Default'],
          ['split', 'Side by side'],
          ['stacked', 'Stacked'],
        ]}
      />
      <Pick
        field="diffWhitespace"
        title="Whitespace in diffs"
        said="Hide lines whose only change is spacing, in a commit's and a card's changes."
        choices={[
          ['show', 'Show'],
          ['ignore', 'Hide'],
        ]}
      />
      <Pick
        field="diffColours"
        title="Diff colours"
        said="Blue and orange instead of green and red, told apart with red-green colour blindness."
        choices={[
          ['standard', 'Green and red'],
          ['safe', 'Blue and orange'],
        ]}
      />
      <Pick
        field="diffCollapsed"
        title="Files in a diff"
        said="Whether each file of a card's or a session's changes opens folded."
        choices={[
          [true, 'Folded'],
          [false, 'Open'],
        ]}
      />
    </>
  )
}
