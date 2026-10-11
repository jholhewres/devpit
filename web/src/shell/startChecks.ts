import type { AgentLaunch, McpServerHealth, MissingSignIn } from '../gen/bindings'

/*
 * What a start says about the sign-ins and servers it runs on. A missing
 * sign-in holds it; a server that is not answering is said, and the session
 * starts anyway — it may never call that server.
 */

/** A held start, as one sentence: each sign-in it would go without, and its fix. */
export function heldWords(missing: readonly MissingSignIn[]): string {
  return `Not started: ${missing.map((one) => `${one.what} — run \`${one.command}\``).join('; ')}.`
}

/** The servers a start that went ahead needs and found not answering, or nothing. */
export function failingWords(failing: readonly McpServerHealth[]): string | null {
  if (failing.length === 0) return null
  const each = failing.map(
    (one) => `${one.name} ${one.state === 'needs_auth' ? 'needs signing in' : 'is not answering'}${one.detail ? ` (${one.detail})` : ''}`,
  )
  return `Started, but the last check found an MCP server it needs down: ${each.join('; ')}. /mcp in the session says more.`
}

/** What a launch has to say, for a caller that can only show a sentence. */
export function launchWords(launch: AgentLaunch): string | null {
  return launch.line === null ? heldWords(launch.missing) : failingWords(launch.failing)
}
