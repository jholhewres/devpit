import { useCallback, useRef } from 'react'

import type { Message } from '../gen/bindings'
import { pagesUntil } from './chatSearch'
import { ask, commands } from './live'
import type { Windowed } from './useWindow'

/* The most messages one page read for a jump asks for: chat.history's own ceiling. */
const JUMP_PAGE = 500

export type Reveal = Windowed['reveal']

/* Goes to a message of the conversation, reading the pages above what is
   drawn when it is not there yet; whether it got there. */
export function useJump(
  where: { projectId: string; conversationId: string } | null,
  chat: { messages: readonly Message[]; older: boolean; took: (page: readonly Message[], older: boolean) => void },
  hold: () => void,
  reveal: React.RefObject<Reveal | null>,
): (id: string, then?: (row: HTMLElement) => void) => Promise<boolean> {
  const latest = useRef(chat)
  latest.current = chat
  return useCallback(
    async (id, then) => {
      const now = latest.current
      if (!now.messages.some((one) => one.id === id)) {
        const first = now.messages[0]?.id
        if (!where || !first || !now.older) return false
        const got = await pagesUntil(id, first, async (before) => {
          const page = await ask(() => commands.chatHistory(where.projectId, where.conversationId, JUMP_PAGE, before))
          return page.data
        })
        if (got.messages.length > 0) now.took(got.messages, got.older)
        if (!got.found) return false
      }
      hold()
      reveal.current?.(id, then)
      return true
    },
    [where, hold, reveal],
  )
}
