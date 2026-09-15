import { useCallback, useEffect, useState } from 'react'

import type { Board, ColumnDeleted, Played, Step } from '../gen/bindings'
import { landed, lanes, type Lane } from './board'
import { shifted } from './laneOrder'
import { ask, commands } from './live'
import { onCarried } from './window'

export interface UseBoard {
  readonly lanes: readonly Lane[]
  readonly error: string | null
  /** Puts the last refusal away once it has been read. */
  dismiss: () => void
  readonly cards: number
  /** The last line each running step printed, by run id. */
  readonly progress: Readonly<Record<string, string>>
  move: (cardId: string, columnId: string, at: number) => void
  /** Runs a card's lane step now. Answers what happened, or null when refused — the refusal is in `error`. */
  play: (cardId: string, confirmed: boolean) => Promise<Played | null>
  addCard: (columnId: string, title: string) => void
  addColumn: (name: string) => void
  renameColumn: (columnId: string, name: string) => void
  reorderColumns: (ids: string[]) => void
  /** Moves a lane one place left or right; nothing at an edge. */
  shiftColumn: (columnId: string, by: -1 | 1) => void
  /** Answers what the backend did: deleted, or refused with the cards in the way. */
  deleteColumn: (columnId: string, moveTo: string | null) => Promise<ColumnDeleted | null>
  /** What a lane runs when a card lands in it, or nothing. */
  setStep: (columnId: string, stepId: string | null) => void
  createStep: (kind: string, name: string, config: string, irreversible: boolean) => void
  /** Where a pass sends a card, and how much the lane decides on its own.
   *  One call for both, because they are one choice. */
  setFlow: (columnId: string, onPass: string | null, autonomy: string) => void
  /** Every step this project has, for the menu that picks one. */
  readonly steps: readonly Step[]
  reload: () => void
}

export function useBoard(projectId: string | null): UseBoard {
  const [board, setBoard] = useState<Board | null>(null)
  const [error, setError] = useState<string | null>(null)

  const reload = useCallback(() => {
    if (!projectId) {
      setBoard(null)
      return
    }
    void ask(() => commands.boardGet(projectId)).then((asked) => {
      setError(asked.error)
      if (asked.data) setBoard(asked.data)
    })
  }, [projectId])

  useEffect(reload, [reload])

  /* A run ends on a thread of its own and says so. Without this the board
     only ever caught up when somebody touched it — a step finishing in the
     background left the tile reading `running` until the next drag. The two
     events were emitted by the backend and nothing listened to either. */
  useEffect(() => onCarried<string>('run:changed', () => reload()), [reload])

  /* What a step is printing as it prints it, by card. Held apart from the
     board rather than folded into it: this arrives many times a second and
     re-reading the whole board on each line would be a board that stutters
     while it works. */
  const [progress, setProgress] = useState<Readonly<Record<string, string>>>({})
  useEffect(
    () =>
      onCarried<[string, string]>('run:progress', ([runId, text]) =>
        setProgress((was) => (was[runId] === text ? was : { ...was, [runId]: text })),
      ),
    [],
  )

  /* The drop shows immediately and is put back if the command refuses — a
     card that snaps to where it was is how you learn the move failed. */
  const move = useCallback(
    (cardId: string, columnId: string, at: number) => {
      if (!projectId || !board) return
      const before = board
      setBoard(landed(board, cardId, columnId, at))
      void ask(() => commands.cardMove(projectId, cardId, columnId, at, false)).then((asked) => {
        setError(asked.error)
        if (asked.error) setBoard(before)
      })
    },
    [board, projectId],
  )

  const play = useCallback(
    async (cardId: string, confirmed: boolean): Promise<Played | null> => {
      if (!projectId) return null
      const asked = await ask(() => commands.cardPlay(projectId, cardId, confirmed))
      setError(asked.error)
      if (asked.data?.run) reload()
      return asked.data
    },
    [projectId, reload],
  )

  const addCard = useCallback(
    (columnId: string, title: string) => {
      if (!projectId) return
      void ask(() => commands.cardCreate(projectId, columnId, title, '')).then((asked) => {
        setError(asked.error)
        if (!asked.error) reload()
      })
    },
    [projectId, reload],
  )

  /* Every column edit is the same shape: call, then reload from the source
     of truth rather than guessing what the backend did. */
  const then = useCallback(
    (call: () => Promise<unknown>) => {
      void ask(call).then((asked) => {
        setError(asked.error)
        if (!asked.error) reload()
      })
    },
    [reload],
  )

  const addColumn = useCallback(
    (name: string) => projectId && then(() => commands.columnCreate(projectId, name)),
    [projectId, then],
  )
  const renameColumn = useCallback(
    (columnId: string, name: string) =>
      projectId && then(() => commands.columnRename(projectId, columnId, name)),
    [projectId, then],
  )
  const reorderColumns = useCallback(
    (ids: string[]) => projectId && then(() => commands.columnReorder(projectId, ids)),
    [projectId, then],
  )
  const shiftColumn = useCallback(
    (columnId: string, by: -1 | 1) => {
      const ids = lanes(board).map((lane) => lane.column.id)
      const order = shifted(ids, columnId, by)
      if (order !== ids) reorderColumns([...order])
    },
    [board, reorderColumns],
  )
  const deleteColumn = useCallback(
    async (columnId: string, moveTo: string | null): Promise<ColumnDeleted | null> => {
      if (!projectId) return null
      const asked = await ask(() => commands.columnDelete(projectId, columnId, moveTo))
      setError(asked.error)
      if (asked.data?.deleted) reload()
      return asked.data
    },
    [projectId, reload],
  )
  const setStep = useCallback(
    (columnId: string, stepId: string | null) =>
      projectId && then(() => commands.columnSetStep(projectId, columnId, stepId)),
    [projectId, then],
  )
  const createStep = useCallback(
    (kind: string, name: string, config: string, irreversible: boolean) =>
      projectId && then(() => commands.stepCreate(projectId, kind, name, config, irreversible)),
    [projectId, then],
  )

  const setFlow = useCallback(
    (columnId: string, onPass: string | null, autonomy: string) =>
      projectId && then(() => commands.columnSetFlow(projectId, columnId, onPass, autonomy)),
    [projectId, then],
  )

  return {
    lanes: lanes(board),
    error,
    dismiss: () => setError(null),
    cards: board?.cards.length ?? 0,
    progress,
    move,
    play,
    addCard,
    addColumn,
    renameColumn,
    reorderColumns,
    shiftColumn,
    deleteColumn,
    setStep,
    createStep,
    setFlow,
    steps: board?.steps ?? [],
    reload,
  }
}
