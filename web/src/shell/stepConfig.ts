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
  'baseRef',
  'project',
  'worktreePath',
  'projectPath',
] as const

/** The keys the form asks for, by kind. Anything else a stored config holds is
 *  somebody else's, and an edit keeps it.
 *
 *  An agent's also counts the names the runner reads as the same field
 *  (`budgetUsd` for `capUsd`, `schema` for `expects`): kept beside the form's,
 *  the pair is a duplicate field and the save is refused. And `skills`, which
 *  the runner refuses outright — kept, it made the step impossible to save
 *  from a form that has no way to remove it. */
const OWNED: Readonly<Record<string, readonly string[]>> = {
  command: ['command', 'timeoutSeconds'],
  session: ['model', 'profile'],
  agent: ['agent', 'profile', 'model', 'capUsd', 'budgetUsd', 'prompt', 'inject', 'expects', 'schema', 'skills'],
}

/** The other name a stored agent config may give a field the form shows. */
const ALSO: Readonly<Record<string, string>> = { capUsd: 'budgetUsd', expects: 'schema' }

/**
 * `stored` is the config being edited. The form's fields are laid over it:
 * a key the form does not ask for survives, and one it asks for and was
 * cleared goes. Rebuilt from the fields alone, an edit dropped every key the
 * form never showed.
 */
export function stepConfig(kind: string, fields: Fields, stored?: string): string {
  const built = fromFields(kind, fields)
  const held = stored === undefined ? null : readObject(stored)
  if (!held) return JSON.stringify(built)
  const owned = OWNED[kind] ?? OWNED.agent!
  const kept = Object.fromEntries(Object.entries(held).filter(([key]) => !owned.includes(key)))
  return JSON.stringify({ ...kept, ...built })
}

function readObject(config: string): Record<string, unknown> | null {
  let read: unknown
  try {
    read = JSON.parse(config)
  } catch {
    return null
  }
  if (typeof read !== 'object' || read === null || Array.isArray(read)) return null
  return read as Record<string, unknown>
}

function fromFields(kind: string, fields: Fields): Record<string, unknown> {
  const text = (key: string): string => (fields[key] ?? '').trim()

  if (kind === 'command') {
    const seconds = Number(text('timeoutSeconds'))
    return {
      command: text('command'),
      // Absent rather than zero: a step with no timeout may take as long as
      // it takes, and `0` would read as "give up at once".
      ...(Number.isFinite(seconds) && seconds > 0 ? { timeoutSeconds: seconds } : {}),
    }
  }

  if (kind === 'session') {
    return {
      ...(text('model') ? { model: text('model') } : {}),
      ...(text('profile') ? { profile: text('profile') } : {}),
    }
  }

  const chosen = text('inject')
    .split(',')
    .map((one) => one.trim())
    .filter(Boolean)
  const cap = Number(text('capUsd'))
  return {
    ...(text('agent') ? { agent: text('agent') } : {}),
    ...(text('profile') ? { profile: text('profile') } : {}),
    ...(text('model') ? { model: text('model') } : {}),
    // Left out when it is not a number of dollars, so the step is refused for
    // having no cap rather than saved with a cap of zero.
    ...(Number.isFinite(cap) && cap > 0 ? { capUsd: cap } : {}),
    prompt: text('prompt'),
    ...(chosen.length ? { inject: chosen } : {}),
    ...(text('expects') ? { expects: text('expects') } : {}),
  }
}

/**
 * The fields behind a stored config, for the form that edits one.
 *
 * `null` when the config cannot be read as an object — a step written by hand
 * in the database, or saved before the form asked for anything. The lane says
 * so rather than opening a form that would quietly replace it.
 */
export function stepFields(kind: string, config: string): Fields | null {
  const held = readObject(config)
  if (!held) return null

  const fields: Record<string, string> = {}
  const put = (key: string): void => {
    const other = ALSO[key]
    const value = held[key] ?? (other === undefined ? undefined : held[other])
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
