import type { AccountQuota, Accounts, QuotaState } from '../gen/bindings'

/*
 * The Claude Code accounts sessions open in, as the screen reads and edits
 * them. Pure, so the rules are tested apart from the pane that draws them.
 */

/** What a quota with no windows says instead of a number. */
export const SAID: Record<Exclude<QuotaState, 'read'>, string> = {
  unavailable: 'unavailable',
  expired: 'open a session in this account',
  signed_out: 'no sign-in in this directory',
}

/** The account a project's new sessions open in, or none. */
export function accountOf(accounts: Accounts, projectId: string): string | null {
  const name = accounts.projects.find((one) => one.projectId === projectId)?.account ?? null
  return name !== null && accounts.accounts.some((one) => one.name === name) ? name : null
}

/** The list with `projectId` opening in `name`; none takes the default away. */
export function withDefault(accounts: Accounts, projectId: string, name: string | null): Accounts {
  const projects = accounts.projects.filter((one) => one.projectId !== projectId)
  return { ...accounts, projects: name === null ? projects : [...projects, { projectId, account: name }] }
}

/** The list without `name`, and without any project default that named it. */
export function without(accounts: Accounts, name: string): Accounts {
  return {
    accounts: accounts.accounts.filter((one) => one.name !== name),
    projects: accounts.projects.filter((one) => one.account !== name),
  }
}

/**
 * Where to suggest the next session goes when this project's account is
 * nearly out: the registered account with the most of its 5-hour window
 * left, among those read and not nearly out themselves.
 */
export function elsewhere(quotas: readonly AccountQuota[], current: string): AccountQuota | null {
  const fiveHour = (one: AccountQuota): number => one.windows.find((window) => window.label === '5-hour')?.percent ?? 0
  return (
    quotas
      .filter((one) => one.name !== current && one.state === 'read' && !one.nearlyOut)
      .sort((a, b) => fiveHour(a) - fiveHour(b))[0] ?? null
  )
}
