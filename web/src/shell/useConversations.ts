import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { ConversationMarks, Profile, Thread } from '../gen/bindings'
import { ask, commands } from './live'
import type { PaneName } from './paneList'
import type { Tab } from './strip'

/*
 * The project's conversations as the sidebar, the menus and the empty chat
 * read them: their marks, their folder, when they were last spoken in.
 */

export const NO_MARKS: ConversationMarks = { name: null, pinned: false, settled: false, snoozedUntil: null, unread: false }

export interface Talks {
  readonly threads: readonly Thread[]
  /** Keeps a change to one conversation's marks; the rows show it at once. */
  mark: (id: string, change: Partial<ConversationMarks>) => void
  /** Reads the list again, once a turn has ended. */
  reload: () => void
  /** A conversation of any project: switched to first, then opened. */
  openChat: (projectId: string, conversationId: string) => void
  /** The agents a conversation can belong to, which name its provider. */
  readonly profiles: readonly Profile[]
  /** What each open chat is doing, as its pane reports it. */
  readonly chatDoing: Readonly<Record<string, 'working' | 'waiting'>>
  tellDoing: (id: string, doing: 'working' | 'waiting' | null) => void
}

export function useConversations(
  projectId: string | null,
  setProject: (projectId: string) => void,
  open: readonly Tab[],
  show: (kind: PaneName, tab?: Partial<Tab>) => void,
): Talks {
  const [threads, setThreads] = useState<readonly Thread[]>([])
  const [asked, setAsked] = useState(0)
  const [wanted, setWanted] = useState<{ projectId: string; conversationId: string } | null>(null)
  const [profiles, setProfiles] = useState<readonly Profile[]>([])
  const [chatDoing, setChatDoing] = useState<Readonly<Record<string, 'working' | 'waiting'>>>({})
  /* A tab opened or closed is a conversation that may have joined the list. */
  const tabs = open.map((tab) => tab.id).join(' ')

  useEffect(() => {
    if (!projectId) return setThreads([])
    let live = true
    void ask(() => commands.chatList(projectId)).then((answer) => live && setThreads(answer.data?.conversations ?? []))
    return () => {
      live = false
    }
  }, [projectId, tabs, asked])

  useEffect(() => {
    void ask(() => commands.agentProfiles()).then((answer) => setProfiles(answer.data ?? []))
  }, [])
  const tellDoing = useCallback((id: string, doing: 'working' | 'waiting' | null) => {
    setChatDoing((was) => {
      if ((was[id] ?? null) === doing) return was
      const next = { ...was }
      if (doing) next[id] = doing
      else delete next[id]
      return next
    })
  }, [])

  /* Read through a ref so `mark` keeps one identity while the list changes. */
  const listedNow = useRef(threads)
  listedNow.current = threads
  const mark = useCallback(
    (id: string, change: Partial<ConversationMarks>) => {
      if (!projectId) return
      /* Not listed yet is a first turn still running: chat.mark refuses only a
         chat with no transcript at all. */
      const marks = { ...NO_MARKS, ...listedNow.current.find((one) => one.id === id)?.marks, ...change }
      setThreads((was) => was.map((one) => (one.id === id ? { ...one, marks } : one)))
      void ask(() => commands.chatMark(projectId, id, marks)).then(() => setAsked((was) => was + 1))
    },
    [projectId],
  )
  const reload = useCallback(() => setAsked((was) => was + 1), [])

  const openChat = useCallback(
    (to: string, conversationId: string) => {
      if (to !== projectId) setProject(to)
      setWanted({ projectId: to, conversationId })
    },
    [projectId, setProject],
  )
  /* The conversation opens in its own project's strip, there only after the switch. */
  useEffect(() => {
    if (!wanted || wanted.projectId !== projectId) return
    show('chat', { id: wanted.conversationId })
    setWanted(null)
  }, [wanted, projectId, show])

  /* One object per change, not per render: it is spread into the shell's context. */
  return useMemo(
    () => ({ threads, mark, reload, openChat, profiles, chatDoing, tellDoing }),
    [threads, mark, reload, openChat, profiles, chatDoing, tellDoing],
  )
}
