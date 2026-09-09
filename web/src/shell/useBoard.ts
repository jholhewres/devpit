import { useCallback, useEffect, useState } from 'react'

import type { Board } from '../gen/bindings'
import { landed, lanes, type Lane } from './board'
import { ask, commands } from './live'

export interface UseBoard {
  readonly lanes: readonly Lane[]
  readonly error: string | null
  readonly cards: number
  move: (cardId: string, columnId: string, at: number) => void
  addCard: (columnId: string, title: string) => void
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

  return { lanes: lanes(board), error, cards: board?.cards.length ?? 0, move, addCard, reload }
}
