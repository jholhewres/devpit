/*
 * Slash commands, offered as they are typed.
 *
 * The list is the CLI's own — read from what it printed when a session of this
 * profile last started — so it is exactly the commands this installation has,
 * plugins included, and never a table kept here that drifts from it.
 */

/* What the composer's text asks for: the word after a leading `/`, while no
   space has been typed yet. After the space it is the command's arguments. */
export function slashQuery(prompt: string): string | null {
  const match = /^\/([^\s/]*)$/.exec(prompt)
  return match ? match[1]! : null
}

/* Past this, a menu is a page. The filter narrows it as more is typed. */
const MOST_SHOWN = 8

/* The commands matching what was typed: those starting with it first, then
   those merely containing it, each group in the CLI's own order. */
export function matching(commands: readonly string[], query: string): readonly string[] {
  const wanted = query.toLowerCase()
  const starts = commands.filter((one) => one.toLowerCase().startsWith(wanted))
  const contains = commands.filter((one) => !one.toLowerCase().startsWith(wanted) && one.toLowerCase().includes(wanted))
  return [...starts, ...contains].slice(0, MOST_SHOWN)
}

/* The composer's text once a command is picked: the command and a space, so
   its arguments can follow without another keystroke. */
export const picked = (command: string): string => `/${command} `

/* Commands devpit answers itself, offered first. `/resume` is one: the CLI
   lists it as a terminal-only command, which the chat leaves out, so a chat
   had no way back into an earlier conversation from where you type. */
export const OWN: readonly string[] = ['resume']

/* An orchestrator's chat answers these from the sessions devpit already
   follows, without a turn. `/sessions`, not `/status`: that one is Claude
   Code's own (version, model, account), and ours would hide it. */
export const ORCHESTRATOR_OWN: readonly string[] = ['sessions', 'waiting', 'ended', 'resume-session']

/* The only one of them that takes words: which ended session to reopen in a
   terminal of its project, where `/resume` brings a conversation into this chat. */
const WITH_WORDS: readonly string[] = ['resume-session']

/* devpit's own commands in a chat; an orchestrator's has its shortcuts too. */
export const ownFor = (orchestrator: boolean): readonly string[] => (orchestrator ? [...OWN, ...ORCHESTRATOR_OWN] : OWN)

/* The menu's list: devpit's own, then the CLI's, once each. */
export const offeredWith = (cli: readonly string[], own: readonly string[] = OWN): readonly string[] => [
  ...own,
  ...cli.filter((one) => !own.includes(one)),
]

/** One of devpit's own commands, with what was typed after it. */
export interface OwnAsked {
  readonly name: string
  readonly arg: string
}

/* The own command a message is, sent as typed rather than picked from the
   menu: `/resume-session api-fix`. Words after a command that takes none make it a
   message for the agent, and so does anything that is not one of `own`. */
export function ownAsked(prompt: string, own: readonly string[]): OwnAsked | null {
  const match = /^\/(\S+)(?:\s+([\s\S]*))?$/.exec(prompt.trim())
  if (!match || !own.includes(match[1]!)) return null
  const name = match[1]!
  const arg = (match[2] ?? '').trim()
  return arg && !WITH_WORDS.includes(name) ? null : { name, arg }
}
