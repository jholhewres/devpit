import type { Part } from '../gen/bindings'
import { kindOf } from './acts'

/*
 * Which of a turn's changed files the agent said it edited.
 *
 * The list itself comes from the checkout before and after the turn, so it is
 * complete. The mark is the difference: a file on that list with no edit call
 * naming it was changed by something the agent ran — a formatter, a script, a
 * `sed` — and that is worth seeing, not smoothing over.
 */

export interface TurnFile {
  readonly path: string
  readonly added: number
  readonly removed: number
}

/* Every path an edit call named, as written by the agent (usually absolute). */
function edited(parts: readonly Part[]): readonly string[] {
  const named: string[] = []
  for (const part of parts) {
    if (part.kind !== 'tool_call' || kindOf(part.name) !== 'edit') continue
    try {
      const args = JSON.parse(part.input) as Record<string, unknown>
      for (const key of ['file_path', 'filePath', 'path', 'notebook_path']) {
        if (typeof args[key] === 'string') named.push(args[key] as string)
      }
    } catch {
      /* Arguments that are not JSON name no file. */
    }
  }
  return named
}

/* Whether the agent's own edit calls account for this file. Compared by the
   end of the path: the checkout reports it relative, the agent absolute. */
export function byAgent(file: TurnFile, parts: readonly Part[]): boolean {
  return edited(parts).some((path) => path === file.path || path.endsWith(`/${file.path}`))
}

export function totals(files: readonly TurnFile[]): { added: number; removed: number } {
  return files.reduce(
    (sum, file) => ({ added: sum.added + file.added, removed: sum.removed + file.removed }),
    { added: 0, removed: 0 },
  )
}
