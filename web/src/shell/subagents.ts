import { useEffect, useState } from 'react'

import type { Happening } from '../gen/bindings'
import { onHappening } from './window'

/*
 * The subagents a terminal's agent started, by pane.
 *
 * Opened by SubagentStart, named when the `Agent` call returns (at launch, in
 * 2.1.270), ended by the first SubagentStop — which the CLI was recorded
 * sending sixteen times for one subagent.
 */

export type Subagent = {
  readonly id: string
  readonly name: string | null
  readonly kind: string | null
  readonly model: string | null
  readonly ended: boolean
}

export type Subagents = Readonly<Record<string, readonly Subagent[]>>

// Ended rows are kept so a late report cannot revive them; this bounds a pane
// whose agent never sent its stops.
const MOST = 32

type Said = { id: string; name?: unknown; kind?: unknown; model?: unknown; ended?: unknown }

function said(detail: string | null): Said | null {
  if (!detail) return null
  try {
    const read: unknown = JSON.parse(detail)
    if (typeof read !== 'object' || read === null) return null
    return typeof (read as Said).id === 'string' ? (read as Said) : null
  } catch {
    return null
  }
}

const text = (value: unknown): string | null => (typeof value === 'string' ? value : null)

export function afterSaid(held: Subagents, paneId: string, detail: string | null): Subagents {
  const one = said(detail)
  if (!one) return held
  const rows = held[paneId] ?? []
  const was = rows.find((row) => row.id === one.id)
  const row: Subagent = {
    id: one.id,
    name: text(one.name) ?? was?.name ?? null,
    kind: text(one.kind) ?? was?.kind ?? null,
    model: text(one.model) ?? was?.model ?? null,
    ended: one.ended === true || (was?.ended ?? false),
  }
  if (
    was &&
    was.name === row.name &&
    was.kind === row.kind &&
    was.model === row.model &&
    was.ended === row.ended
  ) {
    return held
  }
  const next = was
    ? rows.map((each) => (each.id === row.id ? row : each))
    : [...rows, row].slice(-MOST)
  return { ...held, [paneId]: next }
}

/* The subagents still at work in these panes, in the order they started. */
export function runningIn(held: Subagents, paneIds: readonly string[] | undefined): readonly Subagent[] {
  return (paneIds ?? []).flatMap((paneId) => (held[paneId] ?? []).filter((row) => !row.ended))
}

/* `claude-haiku-4-5-20251001` is `haiku-4-5` in a column this narrow. */
export function modelName(model: string): string {
  return model.replace(/^claude-/, '').replace(/-\d{8}$/, '')
}

export function useSubagents(): Subagents {
  const [held, setHeld] = useState<Subagents>({})

  useEffect(
    () =>
      onHappening((happening: Happening) => {
        if (happening.what !== 'subagent') return
        setHeld((was) => afterSaid(was, happening.paneId, happening.detail))
      }),
    [],
  )

  return held
}
