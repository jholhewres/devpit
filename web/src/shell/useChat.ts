import { useCallback, useEffect, useRef, useState } from 'react'

import type { Ask, Attachment, Context, Message, Profile, Question } from '../gen/bindings'
import { applied, ASKS, fixedTo, MODES, send, withFiles } from './chat'
import { ask, commands } from './live'
import { withSkills } from './pills'
import { PROFILES_CHANGED } from './profiles'
import { onPermissionAsked } from './window'
import { useShell } from './useShell'

/** The card a conversation is filed under. */
export interface ChatCard {
  readonly id: string
  readonly title: string | null
  /** Off the board, the chat names it and cannot open it. */
  readonly onBoard: boolean
}

export interface Chat {
  readonly messages: readonly Message[]
  readonly card: ChatCard | null
  readonly profiles: readonly Profile[]
  /** The profile this conversation belongs to, once it has spoken. */
  readonly fixed: string | null
  readonly profileId: string | null
  readonly model: string | null
  /** What every turn so far has cost. */
  readonly cost: number
  /** How full the context was when the last turn ended. */
  readonly context: Context | null
  readonly permission: string
  readonly effort: string | null
  readonly files: readonly Attachment[]
  /** What the agent is waiting to be allowed to do. */
  readonly asked: readonly Question[]
  readonly sending: boolean
  readonly error: string | null
  pick: (profileId: string) => void
  setModel: (model: string | null) => void
  setPermission: (mode: string) => void
  setEffort: (effort: string) => void
  attach: (paths: readonly string[]) => void
  /** Skills picked for the next turn, as pills. */
  readonly skills: readonly string[]
  setSkills: (next: readonly string[]) => void
  /** Keeps a pasted picture and attaches it. */
  paste: (file: Blob) => void
  /** Data URLs for pasted pictures, by the path they were kept under. */
  readonly previews: Readonly<Record<string, string>>
  /** The CLI's id for this conversation, once it has one. */
  readonly session: string | null
  /** Turns a new conversation can go on from, forked at that turn. */
  readonly rewindable: readonly string[]
  rewind: (turnId: string) => void
  detach: (path: string) => void
  answer: (id: string, allow: boolean) => void
  say: (prompt: string) => void
  stop: () => void
}

export function useChat(conversationId: string): Chat {
  const { project, show } = useShell()
  const [messages, setMessages] = useState<readonly Message[]>([])
  const [rewindable, setRewindable] = useState<readonly string[]>([])
  const [skills, setSkills] = useState<readonly string[]>([])
  const [profiles, setProfiles] = useState<readonly Profile[]>([])
  const [fixed, setFixed] = useState<string | null>(null)
  const [profileId, setProfileId] = useState<string | null>(null)
  const [model, setModel] = useState<string | null>(null)
  const [cost, setCost] = useState(0)
  const [context, setContext] = useState<Context | null>(null)
  const [permission, setPermission] = useState(MODES[0].id)
  const [effort, setEffort] = useState<string | null>(null)
  const [files, setFiles] = useState<readonly Attachment[]>([])
  const [asked, setAsked] = useState<readonly Question[]>([])
  const [session, setSession] = useState<string | null>(null)
  const [card, setCard] = useState<ChatCard | null>(null)
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
        setContext(past.data.context ?? null)
        setSession(past.data.sessionId)
        const { cardId, cardTitle, cardOnBoard } = past.data
        setCard(cardId ? { id: cardId, title: cardTitle, onBoard: cardOnBoard } : null)
        setRewindable(past.data.rewindable ?? [])
      }
      const installed = (found.data ?? []).filter((profile) => profile.path !== null)
      setProfiles(installed)
      /* The driver's own default, until someone picks otherwise. */
      setEffort((was) => was ?? installed[0]?.effortDefault ?? null)
      /* Nothing picked and only one account installed: pick it. Asking which
         of one is a question with no answer. */
      setProfileId((was) => was ?? (installed.length === 1 ? installed[0].id : null))
      setError(past.error ?? found.error)
    })()
    return () => {
      live.current = false
    }
  }, [project, conversationId])

  /* A profile saved or switched in Settings is in the picker at once. */
  useEffect(() => {
    const again = (): void =>
      void ask(() => commands.agentProfiles()).then((found) => {
        if (live.current && found.data) setProfiles(found.data.filter((profile) => profile.path !== null))
      })
    window.addEventListener(PROFILES_CHANGED, again)
    return () => window.removeEventListener(PROFILES_CHANGED, again)
  }, [])

  /* The session is told to hold its tools only in the mode that asks. Holding
     them in a mode that never asks would wait for a question nobody sends. */
  useEffect(() => {
    if (!session) return
    void ask(() => commands.permissionAskFromNow(session, ASKS(permission)))
  }, [session, permission])

  /* Questions arrive for every conversation; this one keeps its own. */
  useEffect(() => {
    if (!session) return
    return onPermissionAsked((question) => {
      if (question.sessionId === session) setAsked((was) => [...was, question])
    })
  }, [session])

  const answer = useCallback(
    (id: string, allow: boolean) => {
      const question = asked.find((one) => one.id === id)
      /* Taken off the list first: the question is answered either way, and a
         row that lingers invites a second click that has nothing to answer. */
      setAsked((was) => was.filter((one) => one.id !== id))
      void ask(() => commands.permissionAnswer(id, allow ? 'allow' : 'deny')).then((answered) => {
        /* Kept in the thread only when the answer reached the agent: a receipt
           for a question that had already timed out would record a decision
           nobody's turn ever heard. */
        if (answered.error || !question || !project) return
        void ask(() => commands.chatReceipt(project.id, conversationId, question.tool, question.input, allow)).then(
          (kept) => kept.data && setMessages((was) => [...was, kept.data!]),
        )
      })
    },
    [asked, project, conversationId],
  )

  const say = useCallback(
    (prompt: string) => {
      if (!project || !profileId) return
      const turn: Ask = {
        projectId: project.id,
        conversationId,
        profileId,
        model,
        prompt: withSkills(withFiles(prompt, files), skills),
        cwd: project.rootPath,
        budgetUsd: null,
        permission,
        effort,
      }
      const started = send(turn, (frame) =>
        frame.type === 'session' ? setSession(frame.session_id) : setMessages((was) => applied(was, frame)),
      )
      if (!started) return setError('not running in the app')
      setSending(true)
      setError(null)
      setFiles([])
      /* Picked for one turn, like the attachments: a skill left on would
         silently shape every message after it. */
      setSkills([])
      void started.end
        .then((end) => {
          setCost((was) => was + (end.costUsd ?? 0))
          if (end.context) setContext(end.context)
          /* A turn's place in the CLI's transcript is known once it has run. */
          void ask(() => commands.chatHistory(project.id, conversationId)).then(
            (past) => {
              if (!live.current || !past.data) return
              setRewindable(past.data.rewindable ?? [])
              /* A new conversation learns its session from its first turn; until
                 then nothing could tell it to stop and ask. */
              setSession(past.data.sessionId)
            },
          )
        })
        .catch((thrown: { message?: string }) => setError(thrown.message ?? 'the turn failed'))
        .finally(() => {
          if (!live.current) return
          setSending(false)
          /* A question the turn left behind has nobody waiting on it now. */
          setAsked([])
          /* The first turn settles the account for good. */
          setFixed(profileId)
        })
    },
    [project, conversationId, profileId, model, permission, effort, files, skills],
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

  /* A pasted picture has no path the window can draw, so its preview is kept
     here, by the path the backend gave it. */
  const [previews, setPreviews] = useState<Readonly<Record<string, string>>>({})

  const paste = useCallback(
    (file: Blob) => {
      if (!project) return
      const reader = new FileReader()
      reader.onload = () => {
        const url = typeof reader.result === 'string' ? reader.result : ''
        const data = url.slice(url.indexOf(',') + 1)
        void ask(() => commands.chatPaste(project.id, file.type, data)).then((answer) => {
          if (answer.error) return setError(answer.error)
          const kept = answer.data
          if (!kept) return
          setPreviews((was) => ({ ...was, [kept.path]: url }))
          setFiles((was) => [...was, kept])
        })
      }
      reader.readAsDataURL(file)
    },
    [project],
  )

  const detach = useCallback(
    (path: string) => setFiles((was) => was.filter((file) => file.path !== path)),
    [],
  )

  const rewind = useCallback(
    (turnId: string) => {
      if (!project) return
      void ask(() => commands.chatRewind(project.id, conversationId, turnId)).then((forked) => {
        if (forked.data) show('chat', { id: forked.data })
        else setError(forked.error)
      })
    },
    [project, conversationId, show],
  )

  const stop = useCallback(() => {
    /* The turn's own end clears `sending`: a stop that reached no process yet
       leaves the turn running, and the send button must not come back under it. */
    void ask(() => commands.chatCancel(conversationId))
  }, [conversationId])

  return {
    messages,
    card,
    profiles,
    fixed,
    profileId,
    model,
    cost,
    context,
    permission,
    effort,
    files,
    asked,
    sending,
    error,
    /* Another account is another installation, whose skills are not these:
       a pill picked on one would name nothing on the other. */
    pick: (next: string) => {
      if (next !== profileId) setSkills([])
      setProfileId(next)
    },
    setModel,
    setPermission,
    setEffort,
    attach,
    paste,
    previews,
    skills,
    setSkills,
    session,
    rewindable,
    rewind,
    detach,
    answer,
    say,
    stop,
  }
}
