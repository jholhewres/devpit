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

/**
 * What a step may be told about the card it runs on.
 *
 * The same list as `CONTEXT_KEYS` in `apps/desktop/src/steps/context.rs`,
 * which refuses a key that is not in it. Kept honest by `stepConfig.test.ts`,
 * which reads the Rust one.
 */
export const CONTEXT_KEYS = [
  'card',
  'cardTitle',
  'cardBody',
  'branch',
  'worktreePath',
  'projectPath',
] as const

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

  const chosen = text('inject')
    .split(',')
    .map((one) => one.trim())
    .filter(Boolean)
  const cap = Number(text('capUsd'))
  return JSON.stringify({
    ...(text('agent') ? { agent: text('agent') } : {}),
    ...(text('profile') ? { profile: text('profile') } : {}),
    ...(text('model') ? { model: text('model') } : {}),
    // Left out when it is not a number of dollars, so the step is refused for
    // having no cap rather than saved with a cap of zero.
    ...(Number.isFinite(cap) && cap > 0 ? { capUsd: cap } : {}),
    prompt: text('prompt'),
    ...(chosen.length ? { inject: chosen } : {}),
    ...(text('expects') ? { expects: text('expects') } : {}),
  })
}
