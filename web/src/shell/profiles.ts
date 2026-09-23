import type { Declared, EnvVar, Profile } from '../gen/bindings'

/*
 * A profile, as the pane edits it.
 *
 * The shape is measured rather than designed: five of these existed as shell
 * functions in one `.zshrc` before devpit had anywhere to put them, and every
 * one was environment, a program and some arguments. `glm` and `claude2`
 * differ in nothing but the environment.
 *
 * The parts here are the ones a test can hold: turning a line of arguments
 * into a list and back, what order the rows come in, and whether a draft is
 * finished. The markup reads them.
 */

/*
 * Whether a menu offers this profile. A missing switch reads as on, as it
 * does for `agents.known`; the one already `chosen` stays, so a conversation
 * or a step on an account switched off since still shows what it runs.
 */
export const offers =
  (chosen: string | null) =>
  (profile: Profile): boolean =>
    profile.enabled !== false || profile.id === chosen

/** Arguments as one line, because that is how a person says them. */
export function argsOf(line: string): string[] {
  return line.split(/\s+/).filter((word) => word.length > 0)
}

export function argsText(args: readonly string[]): string {
  return args.join(' ')
}

/*
 * The person's own first, then what devpit noticed.
 *
 * Declared profiles are choices; discovered ones are devpit looking around. A
 * list that mixed them by name would bury the two rows somebody made under ten
 * they did not.
 */
export function ordered(all: readonly Profile[]): readonly Profile[] {
  return [...all].sort((one, two) => {
    if (one.mine !== two.mine) return one.mine ? -1 : 1
    return one.label.localeCompare(two.label)
  })
}

/*
 * A profile being edited.
 *
 * Its own type rather than `Declared`, because `Declared`'s optional fields
 * are optional on the wire — an older stored profile may simply not have had
 * an `env` — and a form with three fields that might not be there is a form
 * every line of which has to say so. Filled in on the way in, emptied back out
 * on the way to the command.
 */
export interface Draft {
  readonly id: string
  readonly label: string
  readonly base: string
  readonly command: string
  readonly args: readonly string[]
  readonly env: readonly EnvVar[]
  /** The models it offers instead of the driver's; empty keeps those. */
  readonly models: readonly string[]
}

/** Whether a draft says enough to be saved. */
export function ready(draft: Draft): boolean {
  return draft.label.trim().length > 0 && draft.base.length > 0
}

/** An existing profile, opened for editing. */
export function draftOf(profile: Profile): Draft {
  return {
    id: profile.id,
    label: profile.label,
    base: profile.base ?? '',
    command: profile.command,
    args: profile.args ?? [],
    env: profile.env ?? [],
    /* Its own list, not `models`: that one has the driver's filled in, and
       saving it back would make the driver's list this profile's forever. */
    models: profile.ownModels ?? [],
  }
}

/** A draft, as the command takes it. */
export function declaredFrom(draft: Draft): Declared {
  return {
    id: draft.id,
    label: draft.label.trim(),
    base: draft.base,
    command: draft.command.trim(),
    args: [...draft.args],
    env: [...draft.env],
    models: [...draft.models],
  }
}

/*
 * A value with its content hidden.
 *
 * An environment value is where a token goes — `glm` carries one — and a
 * settings pane is a thing people screen-share. The length is kept because a
 * blank row and a filled one have to look different.
 */
export function masked(value: string): string {
  return value.length === 0 ? '' : '•'.repeat(Math.min(value.length, 24))
}

/** Whether this variable is one worth hiding by default. */
export function secret(name: string): boolean {
  return /TOKEN|KEY|SECRET|PASSWORD|AUTH/i.test(name)
}

/** A draft for a profile that does not exist yet. */
export function blank(base: string): Draft {
  return { id: '', label: '', base, command: '', args: [], env: [], models: [] }
}

/** The same list with one variable changed, added or dropped. */
export function withVar(
  env: readonly EnvVar[],
  at: number,
  next: EnvVar | null,
): EnvVar[] {
  const copy = [...env]
  if (next === null) copy.splice(at, 1)
  else copy[at] = next
  return copy
}

/*
 * The variables that make a profile another account or another endpoint.
 *
 * `claude2` is `CLAUDE_CONFIG_DIR` and nothing else; `glm` is a base URL and a
 * token. They get fields of their own so nobody has to remember the names, and
 * stay ordinary variables underneath: a profile saved before these fields
 * existed opens with its values already in them.
 */
export const ACCOUNT = {
  config: 'CLAUDE_CONFIG_DIR',
  url: 'ANTHROPIC_BASE_URL',
  token: 'ANTHROPIC_AUTH_TOKEN',
} as const

export const ACCOUNT_VARS: readonly string[] = Object.values(ACCOUNT)

/** Which agents read those variables. Another CLI would ignore them. */
export const hasAccountFields = (base: string): boolean => base === 'claude'

/** One variable's value, or empty when it is not set. */
export function valueOf(env: readonly EnvVar[], name: string): string {
  return env.find((one) => one.name === name)?.value ?? ''
}

/* The list with one named variable set in place, added at the end, or — when
   emptied — dropped: an exported-but-empty `CLAUDE_CONFIG_DIR` is the shape
   that reads as the filesystem root. */
export function withValue(env: readonly EnvVar[], name: string, value: string): EnvVar[] {
  const at = env.findIndex((one) => one.name === name)
  if (value === '') return at < 0 ? [...env] : withVar(env, at, null)
  return at < 0 ? [...env, { name, value }] : withVar(env, at, { name, value })
}

/* The variables the plain list still shows, each with its place in the whole
   list — `withVar` edits by position, and the positions are the full list's. */
export function others(
  env: readonly EnvVar[],
  hidden: readonly string[],
): { readonly one: EnvVar; readonly at: number }[] {
  return env.map((one, at) => ({ one, at })).filter(({ one }) => !hidden.includes(one.name))
}

/* Said when a profile is saved, removed or switched: whatever lists them —
   the chat's picker, the terminal's agents — reads them again, rather than
   showing the list as it was when it opened. */
export const PROFILES_CHANGED = 'devpit:profiles-changed'
export const profilesChanged = (): void => void window.dispatchEvent(new Event(PROFILES_CHANGED))
