import type { Part } from '../gen/bindings'

/*
 * The work a conversation left running beside it.
 *
 * Read from the `task` parts the driver emits — one per change, measured on
 * Claude Code 2.1.270 as `task_started`, `task_progress`, `task_updated` and
 * `task_notification` — so the latest part for a task id is its state.
 */

export interface BackgroundTask {
  readonly id: string
  /** `local_bash` for a command, `local_agent` for a subagent. */
  readonly kind: string | null
  readonly description: string
  readonly status: string
  readonly summary: string | null
}

/* Still going: started, or reporting progress. Anything else is an ending. */
export const running = (task: BackgroundTask): boolean =>
  task.status === 'started' || task.status === 'running'

/* Every task the parts mention, in the order they first appeared, each at its
   latest state. A later part without a description keeps the earlier one: the
   progress and ending lines do not always repeat it. */
export function tasksOf(parts: readonly Part[]): readonly BackgroundTask[] {
  const byId = new Map<string, BackgroundTask>()
  for (const part of parts) {
    if (part.kind !== 'task') continue
    const was = byId.get(part.task_id)
    byId.set(part.task_id, {
      id: part.task_id,
      kind: part.task_kind ?? was?.kind ?? null,
      description: part.description ?? was?.description ?? part.task_id,
      status: part.status,
      summary: part.summary ?? was?.summary ?? null,
    })
  }
  return [...byId.values()]
}

/* How long something has been running, in the words the composer uses. */
export function elapsed(since: number, now: number): string {
  const seconds = Math.max(0, Math.floor((now - since) / 1000))
  return seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`
}
