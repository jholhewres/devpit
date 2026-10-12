import type { DevflowCalibration, RubricAccuracy } from '../gen/bindings'

/*
 * How each rubric Devflow asks Jev agrees with you in this project, learnt
 * from what you did: a card reopened after Jev called it done, a finding
 * called a false positive, a card finished. Jev is never trained; only the
 * line a rubric draws moves, and this says where it would agree most.
 */

const percent = (share: number): string => `${Math.round(share * 100)}%`

export function accuracyLine(one: RubricAccuracy): string {
  let said = `${one.rubric}: ${one.decisions} ${one.decisions === 1 ? 'decision' : 'decisions'}, ${one.labelled} labelled by what you did`
  if (one.accuracy != null) said += `, agreed ${percent(one.accuracy)}${one.min == null ? '' : ` at min ${one.min}`}`
  if (one.suggestedMin != null) {
    said += ` — try min ${one.suggestedMin}${one.suggestedGrey == null ? '' : `, grey ${one.suggestedGrey}`}`
    if (one.suggestedAccuracy != null) said += ` (${percent(one.suggestedAccuracy)})`
  } else if (one.note) said += ` — ${one.note}`
  return `${said}.`
}

export function DevflowAccuracy({ calibration }: { calibration: DevflowCalibration }): React.JSX.Element {
  return (
    <div className="pref pref--stack">
      <div className="pref__body">
        <span className="pref__t">Devflow&rsquo;s rubrics here</span>
        <span className="pref__d">
          Devflow sends Jev each review finding, the card&rsquo;s criteria with a summary of their evidence, the brief and the plan &mdash; never the diff &mdash; through the provider above. A new threshold goes in <code>.devpit/devflow/rubrics/</code>.
        </span>
        {calibration.rubrics.map((one) => (
          <span key={one.rubric} className="pref__d">
            {accuracyLine(one)}
          </span>
        ))}
      </div>
    </div>
  )
}
