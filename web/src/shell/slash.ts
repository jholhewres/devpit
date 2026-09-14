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
