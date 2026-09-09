import { useCallback, useEffect, useRef, useState } from 'react'

import type { Ask, Attachment, Message, Profile } from '../gen/bindings'
import { applied, fixedTo, MODES, send, withFiles } from './chat'
import { ask, commands } from './live'
import { useShell } from './useShell'

export interface Chat {
  readonly messages: readonly Message[]
  readonly profiles: readonly Profile[]
  /** The profile this conversation belongs to, once it has spoken. */
  readonly fixed: string | null
  readonly profileId: string | null
  readonly model: string | null
  /** What every turn so far has cost. */
  readonly cost: number
  readonly permission: string
  readonly files: readonly Attachment[]
  readonly sending: boolean
  readonly error: string | null
  pick: (profileId: string) => void
  setModel: (model: string | null) => void
  setPermission: (mode: string) => void
  attach: (paths: readonly string[]) => void
  detach: (path: string) => void
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
  const [cost, setCost] = useState(0)
  const [permission, setPermission] = useState(MODES[0].id)
  const [files, setFiles] = useState<readonly Attachment[]>([])
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
        setCost(past.data.costUsd ?? 0)
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
        prompt: withFiles(prompt, files),
        cwd: project.rootPath,
        budgetUsd: null,
        permission,
      }
      const started = send(turn, (frame) => setMessages((was) => applied(was, frame)))
      if (!started) return setError('not running in the app')
      setSending(true)
      setError(null)
      setFiles([])
      void started.end
        .then((end) => setCost((was) => was + (end.costUsd ?? 0)))
        .catch((thrown: { message?: string }) => setError(thrown.message ?? 'the turn failed'))
        .finally(() => {
          if (!live.current) return
          setSending(false)
          /* The first turn settles the account for good. */
          setFixed(profileId)
        })
    },
    [project, conversationId, profileId, model, permission, files],
  )

  /* A dropped file is resolved against the project root before it is shown:
     a path outside the project is refused there, not here. */
  const attach = useCallback(
    (paths: readonly string[]) => {
      if (!project) return
      void (async () => {
        const resolved = await Promise.all(
          paths.map((path) => ask(() => commands.chatAttach(project.id, path))),
        )
        const kept = resolved.flatMap((one) => (one.data ? [one.data] : []))
        const refused = resolved.find((one) => one.error)
        if (refused) setError(refused.error)
        setFiles((was) => [
          ...was,
          ...kept.filter((file) => !was.some((had) => had.path === file.path)),
        ])
      })()
    },
    [project],
  )

  const detach = useCallback(
    (path: string) => setFiles((was) => was.filter((file) => file.path !== path)),
    [],
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
    cost,
    permission,
    files,
    sending,
    error,
    pick: setProfileId,
    setModel,
    setPermission,
    attach,
    detach,
    say,
    stop,
  }
}
