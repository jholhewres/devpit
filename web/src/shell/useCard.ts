import { useCallback, useEffect, useState } from 'react'

import type { CardDetail, DeleteRefusal } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * One card, open.
 *
 * Every write answers with the whole card rather than with nothing, so this
 * never has to predict what the backend did — the same reason `settings_write`
 * answers with the settings. A comment added, a file pinned and a date set all
 * come back as the card they changed.
 *
 * `onChanged` is how the board behind this hears about it: the tile shows the
 * deadline and counts the conversation, and both are stale the moment either
 * changes here.
 */

export interface Card {
  readonly detail: CardDetail | null
  readonly error: string | null
  readonly busy: boolean
  save: (title: string, body: string) => void
  setDue: (seconds: number | null) => void
  comment: (body: string) => Promise<string | null>
  editComment: (commentId: string, body: string) => Promise<string | null>
  deleteComment: (commentId: string) => void
  pin: (path: string, label?: string) => Promise<string | null>
  unpin: (pinId: string) => void
  /** Takes it off the board. Answers with why it was refused, or null — the
   *  backend counts the unsaved work and the dialog quotes that count. */
  archive: (force: boolean) => Promise<string | null>
  /** Deletes it. Answers with the refusal or the error, or null once it is gone. */
  remove: (force: boolean) => Promise<DeleteRefusal | string | null>
  reload: () => void
}

export function useCard(
  projectId: string | null,
  cardId: string | null,
  onChanged?: () => void,
): Card {
  const [detail, setDetail] = useState<CardDetail | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const reload = useCallback(() => {
    if (!projectId || !cardId) {
      setDetail(null)
      return
    }
    void ask(() => commands.cardDetail(projectId, cardId)).then((answer) => {
      setError(answer.error)
      if (answer.data) setDetail(answer.data)
    })
  }, [projectId, cardId])

  useEffect(reload, [reload])

  /* Every write is the same shape, and the shape is the point: take the card
     that comes back, and tell the board it changed. Answering with an error
     string rather than throwing, because the field that asked is the one that
     has to keep what was typed. */
  const wrote = useCallback(
    async (call: () => Promise<unknown>): Promise<string | null> => {
      setBusy(true)
      const answer = await ask(call as () => Promise<CardDetail>)
      setBusy(false)
      setError(answer.error)
      if (!answer.data) return answer.error
      setDetail(answer.data)
      onChanged?.()
      return null
    },
    [onChanged],
  )

  const here = (
    call: (project: string, card: string) => Promise<unknown>,
  ): Promise<string | null> =>
    projectId && cardId ? wrote(() => call(projectId, cardId)) : Promise.resolve('no card open')

  return {
    detail,
    error,
    busy,
    save: (title, body) => void here((p, c) => commands.cardUpdate(p, c, title, body)),
    setDue: (seconds) => void here((p, c) => commands.cardSetDue(p, c, seconds)),
    comment: (body) => here((p, c) => commands.cardComment(p, c, body)),
    editComment: (id, body) => here((p, c) => commands.cardCommentEdit(p, c, id, body)),
    deleteComment: (id) => void here((p, c) => commands.cardCommentDelete(p, c, id)),
    pin: (path, label) => here((p, c) => commands.cardPin(p, c, path, label ?? null)),
    unpin: (id) => void here((p, c) => commands.cardUnpin(p, c, id)),
    /* Not through `here`: this answers with the board, not the card, and the
       card it was about is gone by the time it returns. */
    archive: async (force) => {
      if (!projectId || !cardId) return 'no card open'
      const answer = await ask(() => commands.cardArchive(projectId, cardId, force))
      setError(answer.error)
      if (!answer.error) onChanged?.()
      return answer.error
    },
    remove: async (force) => {
      if (!projectId || !cardId) return 'no card open'
      const answer = await ask(() => commands.cardDelete(projectId, cardId, force))
      setError(answer.error)
      if (answer.error) return answer.error
      if (answer.data?.refused) return answer.data.refused
      onChanged?.()
      return null
    },
    reload,
  }
}
