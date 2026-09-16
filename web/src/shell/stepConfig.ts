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

/**
 * The fields behind a stored config, for the form that edits one.
 *
 * `null` when the config cannot be read as an object — a step written by hand
 * in the database, or saved before the form asked for anything. The lane says
 * so rather than opening a form that would quietly replace it.
 */
export function stepFields(kind: string, config: string): Fields | null {
  let read: unknown
  try {
    read = JSON.parse(config)
  } catch {
    return null
  }
  if (typeof read !== 'object' || read === null || Array.isArray(read)) return null
  const held = read as Record<string, unknown>

  const fields: Record<string, string> = {}
  const put = (key: string): void => {
    const value = held[key]
    // A number comes back as what was typed: the form is text, and the config
    // is what `stepConfig` makes of it again.
    if (typeof value === 'string' && value) fields[key] = value
    if (typeof value === 'number') fields[key] = String(value)
  }

  if (kind === 'command') {
    put('command')
    put('timeoutSeconds')
    return fields
  }
  if (kind === 'session') {
    put('model')
    put('profile')
    return fields
  }
  for (const key of ['agent', 'profile', 'model', 'capUsd', 'prompt', 'expects']) put(key)
  const inject = held.inject
  if (Array.isArray(inject)) {
    const keys = inject.filter((one): one is string => typeof one === 'string')
    if (keys.length > 0) fields.inject = keys.join(', ')
  }
  return fields
}
