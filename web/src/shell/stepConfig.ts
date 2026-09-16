/*
 * What the step form saves, as the JSON the runner of that kind reads.
 *
 * A pure function and not a line inside the form: the shape the backend
 * refuses is the one thing the form has to get right, and a rule that only
 * runs on a click is a rule tested by clicking. The cases it is tested
 * against are shared with the Rust side — see
 * `crates/steps/tests/fixtures/step-configs.json`.
 */

/** What the person typed, by field name. */
export type Fields = Readonly<Record<string, string>>

export function stepConfig(kind: string, fields: Fields): string {
  const text = (key: string): string => (fields[key] ?? '').trim()

  if (kind === 'command') {
    const seconds = Number(text('timeoutSeconds'))
    return JSON.stringify({
      command: text('command'),
      // Absent rather than zero: a step with no timeout may take as long as
      // it takes, and `0` would read as "give up at once".
      ...(Number.isFinite(seconds) && seconds > 0 ? { timeoutSeconds: seconds } : {}),
    })
  }

  if (kind === 'session') {
    return JSON.stringify({
      ...(text('model') ? { model: text('model') } : {}),
      ...(text('profile') ? { profile: text('profile') } : {}),
    })
  }

  return JSON.stringify({ agent: text('agent') })
}
