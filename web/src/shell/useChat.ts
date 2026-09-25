import { useCallback, useEffect, useRef, useState } from 'react'

import type { Ask, Attachment, Context, Message, Profile, Question } from '../gen/bindings'
import { applied, ASKS, batched, fixedTo, MODES, rejoin, send, withFiles } from './chat'
import { conversationKey, remember, reopened, type Modes } from './chatModes'
import { ask, commands } from './live'
import { KEPT_BYTES } from './pasting'
import { withSkills } from './pills'
import { offers, onProfilesChanged } from './profiles'
import { onChatWoke, onPermissionAsked } from './window'
import { useShellPick } from './shellStore'

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
  /** The folder the conversation runs in, once a turn has fixed it. */
  readonly folder?: string | null
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

/* What the composer can pick: something devpit can spawn, switched on, or
   the account this conversation already belongs to. */
const pickable = (all: readonly Profile[], belongs: string | null): Profile[] =>
  all.filter((profile) => profile.path !== null).filter(offers(belongs))

export function useChat(conversationId: string): Chat {
  const { project, show } = useShellPick((shell) => ({ project: shell.project, show: shell.show }))
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
  const [folder, setFolder] = useState<string | null>(null)
  const [files, setFiles] = useState<readonly Attachment[]>([])
  /* A pasted picture has no path the window can draw, so its preview is kept
     here, by the path the backend gave it. */
  const [previews, setPreviews] = useState<Readonly<Record<string, string>>>({})
  const [asked, setAsked] = useState<readonly Question[]>([])
  const [session, setSession] = useState<string | null>(null)
  const [card, setCard] = useState<ChatCard | null>(null)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const live = useRef(true)
  /* What the conversation's own head said it last ran with; the profile's
     last pick only fills what that leaves empty. */
  const stored = useRef<Modes>({})

  /* A turn still running when this chat opens — it was left, or the window
     reloaded — is joined where it is, instead of being said to have died. */
  const rejoined = useCallback(
    (projectId: string, woken = false): void => {
      const { push, flush } = batched((frames) => setMessages((was) => frames.reduce(applied, was)))
      const running = rejoin(conversationId, (frame) => {
        if (!live.current) return
        setSending(true)
        if (frame.type === 'session') setSession(frame.session_id)
        else push(frame)
      })
      void running?.then((was) => {
        flush()
        /* A woken turn can be over before this asks to join it — a short
           answer to a message. It was written down; read it from there. */
        if ((!was && !woken) || !live.current) return
        setSending(false)
        // The transcript holds the answer as it was saved, costs and all.
        void ask(() => commands.chatHistory(projectId, conversationId)).then((past) => {
          if (!live.current || !past.data) return
          setMessages(past.data.messages)
          setCost(past.data.costUsd ?? 0)
          setContext(past.data.context ?? null)
          setRewindable(past.data.rewindable ?? [])
        })
      })
    },
    [conversationId],
  )

  useEffect(() => {
    live.current = true
    if (!project) return
    void (async () => {
      const [past, found] = await Promise.all([
        ask(() => commands.chatHistory(project.id, conversationId)),
        ask(() => commands.agentProfiles()),
      ])
      if (!live.current) return
      const belongs = past.data ? fixedTo(past.data) : null
      if (past.data) {
        setMessages(past.data.messages)
        setFixed(belongs)
        setProfileId(belongs)
        setModel(past.data.model)
        setFolder(past.data.cwd ?? null)
        stored.current = { permission: past.data.permission ?? undefined, effort: past.data.effort ?? undefined }
        setCost(past.data.costUsd ?? 0)
        setContext(past.data.context ?? null)
        setSession(past.data.sessionId)
        const { cardId, cardTitle, cardOnBoard } = past.data
        setCard(cardId ? { id: cardId, title: cardTitle, onBoard: cardOnBoard } : null)
        setRewindable(past.data.rewindable ?? [])
        rejoined(project.id)
      }
      const installed = pickable(found.data ?? [], belongs)
      setProfiles(installed)
      /* The driver's own default, until someone picks otherwise. */
      setEffort((was) => was ?? installed[0]?.effortDefault ?? null)
      /* Nothing picked and only one account installed: pick it. Asking which
         of one is a question with no answer. */
      /* An orchestrator speaks as its own account, always. */
      setProfileId((was) => was ?? project.orchestrator ?? (installed.length === 1 ? installed[0].id : null))
      setError(past.error ?? found.error)
    })()
    return () => {
      live.current = false
    }
  }, [project, conversationId, rejoined])

  /* Reopened, a conversation goes back to the mode it ran in; a new one to the
     last pick on its profile, instead of the most careful mode every time. */
  useEffect(() => {
    const open = reopened(conversationId, stored.current, profileId)
    // An orchestrator in the supervised mode would be deaf between turns:
    // its process stays only when nothing waits on a question.
    const permission = open.permission ?? (project?.orchestrator ? 'acceptEdits' : undefined)
    if (permission) setPermission(permission)
    if (open.effort) setEffort(open.effort)
  }, [profileId, conversationId, project?.orchestrator])

  /* Woken by another session, an orchestrator answers without being asked:
     that turn is joined here the way one left running is. */
  useEffect(() => {
    if (!project) return
    return onChatWoke((woken) => {
      if (woken === conversationId) rejoined(project.id, true)
    })
  }, [project, conversationId, rejoined])

  /* A profile saved or switched in Settings is in the picker at once. */
  useEffect(() => {
    const again = (): void =>
      void ask(() => commands.agentProfiles()).then((found) => {
        if (live.current && found.data) setProfiles(pickable(found.data, fixed))
      })
    return onProfilesChanged(again)
  }, [fixed])

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
      const { push, flush } = batched((frames) => setMessages((was) => frames.reduce(applied, was)))
      const started = send(turn, (frame) => (frame.type === 'session' ? setSession(frame.session_id) : push(frame)))
      if (!started) return setError('not running in the app')
      setSending(true)
      setError(null)
      setFiles([])
      setPreviews((was) => {
        for (const shown of Object.values(was)) URL.revokeObjectURL(shown)
        return {}
      })
      /* Picked for one turn, like the attachments: a skill left on would
         silently shape every message after it. */
      setSkills([])
      void started.end
        .then((end) => {
          flush()
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
          /* Whatever is still waiting for a frame is in before the turn is
             said to be over. */
          flush()
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

  const paste = useCallback(
    (file: Blob) => {
      if (!project) return
      // Said here rather than after the whole picture was encoded and sent.
      if (file.size > KEPT_BYTES) return setError('that picture is over 8 MB, too big to attach')
      const reader = new FileReader()
      reader.onload = () => {
        const url = typeof reader.result === 'string' ? reader.result : ''
        const data = url.slice(url.indexOf(',') + 1)
        void ask(() => commands.chatPaste(project.id, file.type, data)).then((answer) => {
          if (answer.error) return setError(answer.error)
          const kept = answer.data
          if (!kept) return
          // Drawn from the blob, not kept as a second copy of it in text.
          const shown = URL.createObjectURL(file)
          setPreviews((was) => ({ ...was, [kept.path]: shown }))
          setFiles((was) => [...was, kept])
        })
      }
      reader.readAsDataURL(file)
    },
    [project],
  )

  const detach = useCallback((path: string) => {
    setFiles((was) => was.filter((file) => file.path !== path))
    setPreviews((was) => {
      if (!(path in was)) return was
      URL.revokeObjectURL(was[path]!)
      const { [path]: _gone, ...rest } = was
      return rest
    })
  }, [])

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
    /* In an orchestrator the one account it belongs to is the only choice. */
    profiles: project?.orchestrator ? profiles.filter((one) => one.id === project.orchestrator) : profiles,
    fixed,
    profileId,
    model,
    cost,
    context,
    permission,
    effort,
    folder,
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
    setPermission: (mode: string) => {
      setPermission(mode)
      remember(conversationKey(conversationId), { permission: mode })
      if (profileId) remember(profileId, { permission: mode })
    },
    setEffort: (next: string) => {
      setEffort(next)
      remember(conversationKey(conversationId), { effort: next })
      if (profileId) remember(profileId, { effort: next })
    },
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
