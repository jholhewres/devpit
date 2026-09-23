import type { KnownAgent, Profile } from '../gen/bindings'
import { told } from './reach'

/*
 * The agent catalogue, as one list the pane can draw.
 *
 * Two sources — the CLIs this build knows, and the profiles the person wrote —
 * joined here rather than in the markup, because the order and the state are
 * the interesting part and the markup should only draw them.
 */

export interface Entry {
  readonly id: string
  readonly label: string
  /** The line this would type, which is the thing a person checks. */
  readonly launch: string
  /** Whether anything on this machine can start it. */
  readonly installed: boolean
  readonly enabled: boolean
  readonly isDefault: boolean
  /** Declared by the person, so it can be edited and removed. */
  readonly mine: boolean
  /** What the editor opens with, for a profile. */
  readonly profile: Profile | null
  /** Where its own documentation lives, when it has one. */
  readonly homepage: string
  /** The agent it behaves like, for the mark it borrows. */
  readonly base: string
  /** Said under the command when there is something to say — a shell function
   *  opens in a terminal and nowhere else, and a row that only showed the
   *  command would let somebody put it on a board and wait. */
  readonly note: string | null
}

/*
 * Installed first, then the person's own, then by name.
 *
 * Installed first because the list exists to answer "what can I start", and an
 * agent that is not here cannot be started however well it sorts. The default
 * is not hoisted: a row that moves when you pick it is a row you then have to
 * find again.
 */
export function catalogue(
  agents: readonly KnownAgent[],
  profiles: readonly Profile[],
  choice: { defaultId: string; disabled?: readonly string[] },
): readonly Entry[] {
  /* The choice comes back from every toggle, while `agents` was asked when
     the pane opened, so the choice is the one that knows. */
  const on = (id: string, said: boolean): boolean => (choice.disabled ? !choice.disabled.includes(id) : said)

  const mine: Entry[] = profiles
    .filter((profile) => profile.mine)
    .map((profile) => ({
      id: profile.id,
      label: profile.label,
      launch: profile.command,
      installed: profile.reach !== 'missing',
      enabled: on(profile.id, profile.enabled !== false),
      isDefault: choice.defaultId === profile.id,
      mine: true,
      profile,
      homepage: '',
      base: profile.base ?? '',
      note: profile.reach === 'runnable' ? null : told(profile).sub,
    }))

  const built: Entry[] = agents
    /* A profile is listed once. `agents.known` already puts declared profiles
       at the front of its own answer, and drawing both would be the same row
       twice with two different buttons on it. */
    .filter((agent) => !mine.some((one) => one.id === agent.id))
    .map((agent) => ({
      id: agent.id,
      label: agent.label,
      launch: agent.launch,
      installed: agent.installed,
      /* The choice as it stands now decides — it is what the switch just
         changed; the catalogue's own flag was read once and goes stale.
         Absent reads as offered: an older answer that does not carry it must
         not hide every agent on the list. */
      enabled: on(agent.id, agent.enabled !== false),
      isDefault: choice.defaultId === agent.id,
      mine: false,
      profile: null,
      homepage: agent.homepage ?? '',
      base: agent.id,
      note: agent.installed ? null : 'not on this machine',
    }))

  const by = (one: Entry, two: Entry): number =>
    one.installed !== two.installed
      ? Number(two.installed) - Number(one.installed)
      : one.label.localeCompare(two.label)

  return [...[...mine].sort(by), ...[...built].sort(by)]
}

/** How many of these this machine can actually start. */
export const detected = (entries: readonly Entry[]): number =>
  entries.filter((entry) => entry.installed).length

/*
 * What the default picker offers.
 *
 * Only what can be started and has not been switched off: a default pointing
 * at something absent opens nothing, and one pointing at something hidden is a
 * menu whose default is not in it.
 */
export const choosable = (entries: readonly Entry[]): readonly Entry[] =>
  entries.filter((entry) => entry.installed && entry.enabled)

/** Whether the stored default still names something choosable. */
export const defaultLost = (entries: readonly Entry[], defaultId: string): boolean =>
  defaultId !== '' && !choosable(entries).some((entry) => entry.id === defaultId)

/*
 * The two halves of the list.
 *
 * "Installed" saying `not on this machine` under a row is the heading arguing
 * with its own contents. What is here and what could be here are two answers,
 * so they are two groups.
 */
export const here = (entries: readonly Entry[]): readonly Entry[] =>
  entries.filter((entry) => entry.installed)

export const elsewhere = (entries: readonly Entry[]): readonly Entry[] =>
  entries.filter((entry) => !entry.installed)
