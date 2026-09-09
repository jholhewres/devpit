import { useCallback, useEffect, useRef, useState } from 'react'

import type { Ask, Message, Profile } from '../gen/bindings'
import { applied, fixedTo, send } from './chat'
import { ask, commands } from './live'
import { useShell } from './useShell'

export interface Chat {
  readonly messages: readonly Message[]
  readonly profiles: readonly Profile[]
  /** The profile this conversation belongs to, once it has spoken. */
  readonly fixed: string | null
  readonly profileId: string | null
  readonly model: string | null
  readonly sending: boolean
  readonly error: string | null
  pick: (profileId: string) => void
  setModel: (model: string | null) => void
  say: (prompt: string) => void
  stop: () => void
}

export function useChat(conversationId: string): Chat {
  const { project } = useShell()
  const [messages, setMessages] = useState<readonly Message[]>([])
  const [profiles, setProfiles] = useState<readonly Profile[]>([])
  const [fixed, setFixed] = useState<string | null>(null)
  const [profileId, setProfileId] = useState<string | null>(null)
  const [model, setModel] = useState<string | null>(null)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const live = useRef(true)

  useEffect(() => {
    live.current = true
    if (!project) return
    void (async () => {
      const [past, found] = await Promise.all([
        ask(() => commands.chatHistory(project.id, conversationId)),
        ask(() => commands.agentProfiles()),
      ])
      if (!live.current) return
      if (past.data) {
        setMessages(past.data.messages)
        const belongs = fixedTo(past.data)
        setFixed(belongs)
        setProfileId(belongs)
        setModel(past.data.model)
      }
      const installed = (found.data ?? []).filter((profile) => profile.path !== null)
      setProfiles(installed)
      /* Nothing picked and only one account installed: pick it. Asking which
         of one is a question with no answer. */
      setProfileId((was) => was ?? (installed.length === 1 ? installed[0].id : null))
      setError(past.error ?? found.error)
    })()
    return () => {
      live.current = false
    }
  }, [project, conversationId])

  const say = useCallback(
    (prompt: string) => {
      if (!project || !profileId) return
      const turn: Ask = {
        projectId: project.id,
        conversationId,
        profileId,
        model,
        prompt,
        cwd: project.rootPath,
        budgetUsd: null,
      }
      const started = send(turn, (frame) => setMessages((was) => applied(was, frame)))
      if (!started) return setError('not running in the app')
      setSending(true)
      setError(null)
      void started.end
        .catch((thrown: { message?: string }) => setError(thrown.message ?? 'the turn failed'))
        .finally(() => {
          if (!live.current) return
          setSending(false)
          /* The first turn settles the account for good. */
          setFixed(profileId)
        })
    },
    [project, conversationId, profileId, model],
  )

  const stop = useCallback(() => {
    void ask(() => commands.chatCancel(conversationId)).then(() => setSending(false))
  }, [conversationId])

  return {
    messages,
    profiles,
    fixed,
    profileId,
    model,
    sending,
    error,
    pick: setProfileId,
    setModel,
    say,
    stop,
  }
}
