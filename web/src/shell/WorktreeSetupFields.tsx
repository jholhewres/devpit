import type { EnvVar, WorktreeSetup } from '../gen/bindings'

/*
 * What a card's own checkout gets before anyone works in it: the `.env` a
 * fresh worktree does not have, the install it needs. One entry per line,
 * the way a person writes them in a README.
 */

const lines = (text: string): string[] => text.split('\n')
const text = (all: readonly string[]): string => all.join('\n')

/** `NAME=value` per line, as `share` is kept. */
export function envOf(typed: string): EnvVar[] {
  return lines(typed)
    .map((line) => line.trim())
    .filter((line) => line.includes('='))
    .map((line) => ({ name: line.slice(0, line.indexOf('=')).trim(), value: line.slice(line.indexOf('=') + 1) }))
    .filter((one) => one.name.length > 0)
}

export function WorktreeSetupFields({
  setup,
  onChange,
}: {
  setup: WorktreeSetup
  onChange: (next: WorktreeSetup) => void
}): React.JSX.Element {
  return (
    <details className="pdlg__f wtsetup" open={setup.copy.length + setup.run.length > 0}>
      <summary>New worktree setup</summary>
      <p className="wtsetup__d">What a card&rsquo;s own checkout gets before anyone works in it. Kept on this machine, never committed.</p>
      <label className="wtsetup__f">
        <span>Copy from the main checkout</span>
        <textarea rows={2} placeholder={'.env*\nconfig/*.local.json'} value={text(setup.copy)} onChange={(event) => onChange({ ...setup, copy: lines(event.target.value) })} />
      </label>
      <label className="wtsetup__f">
        <span>Link to the main checkout</span>
        <textarea rows={1} placeholder="data/fixtures" value={text(setup.link)} onChange={(event) => onChange({ ...setup, link: lines(event.target.value) })} />
      </label>
      <label className="wtsetup__f">
        <span>Run, in order</span>
        <textarea rows={2} placeholder={'pnpm install\nmake setup'} value={text(setup.run)} onChange={(event) => onChange({ ...setup, run: lines(event.target.value) })} />
      </label>
      <label className="wtsetup__f">
        <span>Variables for those commands</span>
        <textarea
          rows={1}
          placeholder="CARGO_TARGET_DIR=/tmp/shared-target"
          defaultValue={setup.share.map((one) => `${one.name}=${one.value}`).join('\n')}
          onChange={(event) => onChange({ ...setup, share: envOf(event.target.value) })}
        />
      </label>
    </details>
  )
}
