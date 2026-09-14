import type { Installation, OutsideSession, Profile } from '../gen/bindings'

/*
 * Which profile opens a session started outside devpit.
 *
 * A session belongs to the installation that wrote it, and resuming it under
 * another installation's account would start somewhere with none of its
 * history. So the profile has to run against that same configuration
 * directory: one of the profiles the installation lists by name, or — for the
 * installation no profile names — the CLI devpit found on its own.
 */

export function profileFor(
  session: OutsideSession,
  installations: readonly Installation[],
  profiles: readonly Profile[],
): Profile | null {
  return profileForInstallation(session.installation, installations, profiles)
}

/* The same rule, for a session known only by the installation folder it is in. */
export function profileForInstallation(
  directory: string,
  installations: readonly Installation[],
  profiles: readonly Profile[],
): Profile | null {
  const installation = installations.find((one) => one.directory === directory)
  if (!installation) return null
  const runnable = profiles.filter((profile) => profile.reach === 'runnable' && profile.driver === 'claude')
  if (installation.profiles.length > 0) {
    return runnable.find((profile) => installation.profiles.includes(profile.label)) ?? null
  }
  // No profile names this installation: it is the one a plain `claude` uses.
  return runnable.find((profile) => !profile.mine) ?? null
}

/* What a session is called in the list: the CLI's title, or that it has none. */
export const titled = (session: OutsideSession): string => session.title ?? 'Untitled session'
