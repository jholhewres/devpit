import { useCallback, useEffect, useRef, useState } from 'react'

import type { AgentHealth, Board, Conversation, Conversations, RemoteDraft, RemoteIn, RemoteOut, RemoteProject, RemoteQuestion } from '../gen/bindings'
import { RemoteBoard } from './RemoteBoard'
import { RemoteChats } from './RemoteChats'
import { RemotePair } from './RemotePair'
import { RemoteTerminal } from './RemoteTerminal'
import { RemoteWaiting } from './RemoteWaiting'
import { connect, keepToken, tokenKept, type Link } from './socket'

/*
 * The viewer: which machine, which project, and its terminals, board,
 * questions and chats. What it may do — type, answer — is the machine's word,
 * said in its welcome; asking past it is refused there, not only here.
 */

type View = 'terminals' | 'board' | 'waiting' | 'chats'

export interface Welcome {
  device: string
  host: string
  typing: boolean
  answering: boolean
}

export function RemoteApp(): React.JSX.Element {
  const [token, setToken] = useState<string | null>(tokenKept)
  const [up, setUp] = useState(false)
  const [welcome, setWelcome] = useState<Welcome | null>(null)
  const [refused, setRefused] = useState<string | null>(null)
  const [failed, setFailed] = useState<string | null>(null)
  const [projects, setProjects] = useState<readonly RemoteProject[]>([])
  const [projectId, setProjectId] = useState<string | null>(null)
  const [view, setView] = useState<View>('terminals')
  const [pane, setPane] = useState<string | null>(null)
  const [board, setBoard] = useState<Board | null>(null)
  const [questions, setQuestions] = useState<readonly RemoteQuestion[]>([])
  const [drafts, setDrafts] = useState<readonly RemoteDraft[]>([])
  const [health, setHealth] = useState<AgentHealth | null>(null)
  const [chats, setChats] = useState<Conversations | null>(null)
  const [chat, setChat] = useState<Conversation | null>(null)
  const link = useRef<Link | null>(null)
  const bytes = useRef<(pane: string, b64: string) => void>(() => {})
  const projectRef = useRef(projectId)
  projectRef.current = projectId

  const send = useCallback((message: RemoteIn) => link.current?.send(message), [])

  useEffect(() => {
    if (!token) return
    const heard = (message: RemoteOut): void => {
      switch (message.t) {
        case 'welcome':
          setWelcome(message)
          setRefused(null)
          link.current?.send({ t: 'projects' })
          link.current?.send({ t: 'waiting' })
          link.current?.send({ t: 'drafts' })
          return
        case 'refused':
          setRefused(message.why)
          return
        case 'projects':
          setProjects(message.projects)
          setProjectId((was) => was ?? message.projects.find((one) => one.terminals.length > 0)?.id ?? message.projects[0]?.id ?? null)
          return
        case 'paneBytes':
          bytes.current(message.pane, message.b64)
          return
        case 'paneClosed':
          setPane((was) => (was === message.pane ? null : was))
          return
        case 'board':
          if (message.project === projectRef.current) setBoard(message.board)
          return
        case 'boardChanged':
          if (message.project === projectRef.current) link.current?.send({ t: 'board', project: message.project })
          return
        case 'waiting':
          setQuestions(message.questions)
          return
        case 'drafts':
          setDrafts(message.drafts)
          return
        case 'agentHealth':
          setHealth(message.health)
          return
        case 'chats':
          if (message.project === projectRef.current) setChats(message.conversations)
          return
        case 'chat':
          setChat(message.conversation)
          return
        case 'failed':
          setFailed(message.why)
          window.setTimeout(() => setFailed(null), 6000)
          return
        case 'pong':
          return
      }
    }
    link.current = connect(token, heard, setUp)
    return () => link.current?.close()
  }, [token])

  /* Each view asks for what it shows when it is opened. */
  useEffect(() => {
    if (!projectId || !up) return
    if (view === 'board') send({ t: 'board', project: projectId })
    if (view === 'chats') send({ t: 'chats', project: projectId })
    if (view === 'terminals') send({ t: 'projects' })
    if (view === 'waiting') {
      send({ t: 'waiting' })
      send({ t: 'drafts' })
      send({ t: 'agentHealth' })
    }
  }, [view, projectId, up, send])

  if (!token) return <RemotePair onPaired={(fresh) => (keepToken(fresh), setToken(fresh))} />

  if (refused)
    return (
      <main className="rm rm--center">
        <h1 className="rm__title">Not let in</h1>
        <p className="rm__said">{refused}</p>
        <button className="rm__btn" onClick={() => (keepToken(null), setToken(null), setRefused(null))}>
          Pair again
        </button>
      </main>
    )

  const project = projects.find((one) => one.id === projectId) ?? null
  return (
    <main className="rm">
      <header className="rm__head">
        <span className="rm__dot" data-up={up ? 'true' : undefined} />
        <span className="rm__host">{welcome?.host ?? 'devpit'}</span>
        <select className="rm__pick" aria-label="Project" value={projectId ?? ''} onChange={(event) => (setProjectId(event.target.value), setPane(null), setBoard(null), setChats(null), setChat(null))}>
          {projects.map((one) => (
            <option key={one.id} value={one.id}>
              {one.name}
            </option>
          ))}
        </select>
      </header>
      {failed && <p className="rm__failed" role="alert">{failed}</p>}
      <nav className="rm__tabs">
        {(['terminals', 'board', 'waiting', 'chats'] as const).map((one) => (
          <button key={one} className="rm__tab" data-on={view === one ? 'true' : undefined} onClick={() => setView(one)}>
            {one === 'waiting' && questions.length + drafts.length > 0 ? `waiting ${questions.length + drafts.length}` : one}
          </button>
        ))}
      </nav>
      <section className="rm__body">
        {view === 'terminals' && project && (
          <>
            <div className="rm__list">
              {project.terminals.length === 0 && <p className="rm__said">No terminal is open in {project.name}.</p>}
              {project.terminals.map((one) => (
                <button key={one.pane} className="rm__row" data-on={pane === one.pane ? 'true' : undefined} onClick={() => setPane(one.pane)}>
                  <span>{one.command || 'shell'}</span>
                  <span className="rm__dim">{one.pane.slice(-6)}</span>
                </button>
              ))}
            </div>
            {pane && <RemoteTerminal key={pane} project={project.id} pane={pane} typing={welcome?.typing ?? false} send={send} bytes={bytes} />}
          </>
        )}
        {view === 'board' && <RemoteBoard board={board} project={projectId} typing={welcome?.typing ?? false} send={send} />}
        {view === 'waiting' && <RemoteWaiting questions={questions} drafts={drafts} health={health} answering={welcome?.answering ?? false} typing={welcome?.typing ?? false} send={send} />}
        {view === 'chats' && projectId && <RemoteChats project={projectId} chats={chats} chat={chat} onClose={() => setChat(null)} send={send} />}
      </section>
    </main>
  )
}
