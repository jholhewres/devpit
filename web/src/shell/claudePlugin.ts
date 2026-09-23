import type { ClaudePluginInstallation } from '../gen/bindings'

/*
 * Whether the pane's footer offers the devpit plugin for Claude Code, and in
 * which words.
 *
 * Any installation behind is enough: which one the agent in this pane runs
 * against is not something devpit reads off a running process, and the
 * install brings every one of them up to date at once.
 */
export type Offer = 'install' | 'update' | null

export function offerFor(installations: readonly ClaudePluginInstallation[]): Offer {
  if (installations.some((one) => one.state === 'missing')) return 'install'
  if (installations.some((one) => one.state === 'outdated')) return 'update'
  return null
}

const DISMISSED = 'devpit.claudePlugin.dismissed'

/* Dismissed for one offer: a later build that changes the plugin asks again. */
export function dismissed(offer: Offer, installations: readonly ClaudePluginInstallation[]): boolean {
  try {
    return offer !== null && localStorage.getItem(DISMISSED) === signature(installations)
  } catch {
    return false
  }
}

export function dismiss(installations: readonly ClaudePluginInstallation[]): void {
  try {
    localStorage.setItem(DISMISSED, signature(installations))
  } catch {
    /* A private window forgets it; the chip comes back, nothing breaks. */
  }
}

const signature = (installations: readonly ClaudePluginInstallation[]): string =>
  installations.map((one) => `${one.directory}:${one.state}`).join('|')
