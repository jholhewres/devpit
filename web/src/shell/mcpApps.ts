import { createContext } from 'react'

import { ask, commands } from './live'

/*
 * Which tools of a conversation's account, in its folder, come with a page.
 *
 * Asked only when an answer used an MCP tool at all, and once per account and
 * folder for the window's life: finding out starts a process of the CLI that
 * connects every server of the account, which is too dear to do for every
 * chat that is merely opened.
 */

export interface AppsScope {
  readonly profileId: string
  readonly cwd: string
}

export const AppsScopeContext = createContext<AppsScope | null>(null)

/** Whether the page draws its own frame, by the tool that brings it. */
export type AppTools = ReadonlyMap<string, { bordered: boolean }>

const asked = new Map<string, Promise<AppTools>>()

export function appTools(scope: AppsScope): Promise<AppTools> {
  const key = `${scope.profileId}\n${scope.cwd}`
  let found = asked.get(key)
  if (!found) {
    found = ask(() => commands.mcpAppTools(scope.profileId, scope.cwd)).then((answer) => {
      // A refusal is not kept: the next answer that used a tool asks again.
      if (!answer.data) asked.delete(key)
      return new Map((answer.data ?? []).map((one) => [one.called, { bordered: one.bordered }]))
    })
    asked.set(key, found)
  }
  return found
}

/** The name the model calls an MCP tool by. */
export const isMcp = (name: string): boolean => name.startsWith('mcp__')
