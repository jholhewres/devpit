import { useCallback, useEffect, useMemo, useRef } from 'react'

import type { Message } from '../gen/bindings'
import { ask, commands } from './live'
import { useFollow, type Follow } from './useFollow'

/* How many messages a chat opens with, and adds each time it is scrolled up. */
export const PAGE = 50

/* The messages before `first`, prepended; the scroller keeps showing what it
   showed, measured from the bottom, since everything grew above it. */
export function useOlder(
  where: { projectId: string; conversationId: string } | null,
  messages: readonly Message[],
  older: boolean,
  took: (page: readonly Message[], older: boolean) => void,
  box: React.RefObject<HTMLElement | null>,
): React.RefObject<HTMLDivElement | null> {
  const top = useRef<HTMLDivElement>(null)
  const asking = useRef(false)
  const first = messages[0]?.id ?? null
  /* Height under the top of the view, before a page went in above it. */
  const below = useRef<number | null>(null)

  const more = useCallback(() => {
    if (!where || !first || asking.current) return
    asking.current = true
    const scroll = box.current
    void ask(() => commands.chatHistory(where.projectId, where.conversationId, PAGE, first)).then((page) => {
      asking.current = false
      if (!page.data) return
      below.current = scroll ? scroll.scrollHeight - scroll.scrollTop : null
      took(page.data.messages, page.data.older)
    })
  }, [where, first, box, took])

  /* Back where it was, once the page above is drawn. */
  useEffect(() => {
    const scroll = box.current
    if (!scroll || below.current === null) return
    scroll.scrollTop = scroll.scrollHeight - below.current
    below.current = null
  }, [first, box])

  useEffect(() => {
    const at = top.current
    if (!at || !older || typeof IntersectionObserver === 'undefined') return
    const watch = new IntersectionObserver((seen) => seen.some((one) => one.isIntersecting) && more(), {
      root: box.current,
    })
    watch.observe(at)
    return () => watch.disconnect()
  }, [older, more, box])

  return top
}

/* The thread's scroller. New output belongs at the bottom, where the eye
   already is, unless it went up to read; older messages load at the top. */
export function useThread(
  projectId: string | null,
  conversationId: string,
  chat: { messages: readonly Message[]; older: boolean; took: (page: readonly Message[], older: boolean) => void },
): Follow<HTMLDivElement> & { top: React.RefObject<HTMLDivElement | null> } {
  const follow = useFollow<HTMLDivElement>(chat.messages)
  const where = useMemo(() => (projectId ? { projectId, conversationId } : null), [projectId, conversationId])
  const top = useOlder(where, chat.messages, chat.older, chat.took, follow.box)
  return { ...follow, top }
}
