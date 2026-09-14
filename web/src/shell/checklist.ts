import type { Part } from '../gen/bindings'

/*
 * The agent's checklist, rebuilt from the calls that keep it.
 *
 * Claude Code 2.1.270 keeps it in `TaskCreate` ({subject, description}, whose
 * result names the id it was given) and `TaskUpdate` ({taskId, status}) — not
 * `TodoWrite`, which it no longer calls. The checklist is the replay of those.
 */

export interface Item {
  readonly id: string
  readonly subject: string
  readonly status: string
}

export interface Checklist {
  readonly items: readonly Item[]
  /** Ids whose status the most recent update changed. */
  readonly changed: readonly string[]
}

function parsed(text: string): Record<string, unknown> | null {
  try {
    const value: unknown = JSON.parse(text)
    return typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : null
  } catch {
    return null
  }
}

const text = (value: unknown): string => (typeof value === 'string' ? value : '')

/* The id a create was given, read from its result: `{"task":{"id":"1",…}}`,
   or a line of prose naming `#1` when the CLI answers in words. */
function createdId(output: string): string | null {
  const task = parsed(output)?.task
  if (task && typeof task === 'object' && typeof (task as Record<string, unknown>).id === 'string') {
    return (task as Record<string, unknown>).id as string
  }
  return /#(\d+)/.exec(output)?.[1] ?? null
}

export function checklistOf(parts: readonly Part[]): Checklist {
  const results = new Map<string, string>()
  for (const part of parts) {
    if (part.kind === 'tool_result' && !part.is_error) results.set(part.call_id, part.output)
  }

  const items = new Map<string, Item>()
  let changed: string[] = []
  for (const part of parts) {
    if (part.kind !== 'tool_call' || part.parent) continue
    const args = parsed(part.input)
    if (!args) continue
    if (part.name === 'TaskCreate') {
      const id = createdId(results.get(part.id) ?? '')
      if (!id) continue
      items.set(id, { id, subject: text(args.subject) || text(args.description), status: 'pending' })
    } else if (part.name === 'TaskUpdate') {
      const id = text(args.taskId)
      const item = items.get(id)
      if (!item || !text(args.status) || text(args.status) === item.status) continue
      items.set(id, { ...item, status: text(args.status) })
      changed = [id]
    }
  }
  return { items: [...items.values()], changed }
}

export const done = (item: Item): boolean => item.status === 'completed'
