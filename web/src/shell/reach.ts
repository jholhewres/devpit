import type { Profile } from '../gen/bindings'

/*
 * How far this machine gets with a command, said out loud.
 *
 * Three answers, because "installed" was a yes/no and it was wrong about the
 * machine it was written on: `claudin` there is a shell function, so the row
 * said "not on the PATH" about a command its owner watches their terminal run
 * every day. The terminal types into their shell and can start it; devpit
 * spawns a process and cannot.
 *
 * Its own file because the rows and their tests both read it, and a copy in
 * each is a copy that disagrees.
 */

export interface Told {
  /** The colour of the dot on the badge. */
  readonly dot: string
  /** The line under the name. */
  readonly sub: string
  /** Whether the name is dimmed. */
  readonly off: boolean
}

export function told(profile: Profile): Told {
  switch (profile.reach) {
    case 'runnable':
      return {
        dot: 'var(--success)',
        sub: profile.path ?? '',
        off: false,
      }
    case 'shell_only':
      return {
        dot: 'var(--warning)',
        // Named as the thing it is, because the fix follows from the name:
        // point a profile at the program the function runs.
        sub: 'a shell function — opens in the terminal, not on the board',
        off: false,
      }
    case 'missing':
      return { dot: 'var(--ghost)', sub: 'not found', off: true }
  }
}

/** How many models this profile offers, when it offers any. */
export function models(profile: Profile): string {
  const count = (profile.models ?? []).length
  return count === 0 ? '' : `${count} model${count === 1 ? '' : 's'}`
}
