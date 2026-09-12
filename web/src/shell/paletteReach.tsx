import { useEffect, useMemo, useState } from 'react'

import { AgentMark } from './AgentMark'
import type { Card, KnownAgent } from '../gen/bindings'
import { ask, commands } from './live'
import { PANES, type PaneName } from './paneList'
import type { Row } from './paletteGroups'
import { GEAR } from './paletteIcons'
import { useShell } from './useShell'

/*
 * What the field can reach, gathered.
 *
 * Apart from the field itself because they are two jobs: this asks the
 * backend what exists and turns it into rows, and `Palette` draws a list and
 * moves a highlight through it. The field is the part that is fiddly to read;
 * it should not also be the part that is long.
 */

const SHORTCUT: Partial<Record<PaneName, string>> = { chat: '⌘N', term: '⌘T', board: '⌘B' }

export interface Reachable {
  readonly panes: Row[]
  readonly agents: Row[]
  readonly sessions: Row[]
  readonly cards: readonly Card[]
  readonly files: readonly string[]
  readonly indexing: boolean
  /** The file list stopped short of the whole project. */
  readonly partial: boolean
}

export function useReachable(): Reachable {
  const { show, openPrefs, focus, open: tabs, project } = useShell()
  const [files, setFiles] = useState<readonly string[]>([])
  const [indexing, setIndexing] = useState(true)
  const [partial, setPartial] = useState(false)
  const [cards, setCards] = useState<readonly Card[]>([])
  const [agents, setAgents] = useState<readonly KnownAgent[]>([])

  /* Which agents are on this machine. Asked when the field opens rather than
     kept: a CLI installed while the window was up should appear the next time
     the menu is used, not the next time the app is started. */
  useEffect(() => {
    void ask(() => commands.agentsKnown()).then((asked) => setAgents(asked.data ?? []))
  }, [])

  /* Once, when the field opens. */
  useEffect(() => {
    if (!project) return setIndexing(false)
    void Promise.all([
      ask(() => commands.projectFiles(project.id, null)),
      ask(() => commands.boardGet(project.id)),
    ]).then(([listed, board]) => {
      setFiles(listed.data?.paths ?? [])
      setPartial(listed.data?.partial ?? false)
      setCards(board.data?.cards ?? [])
      setIndexing(false)
    })
  }, [project])

  const panes: Row[] = useMemo(
    () => [
      ...PANES.filter((pane) => pane.name !== 'file' && pane.name !== 'diff').map((pane) => ({
        key: `pane:${pane.name}`,
        name: pane.title,
        meta: SHORTCUT[pane.name] ?? '',
        icon: pane.icon,
        go: () => show(pane.name),
      })),
      {
        key: 'prefs',
        name: 'Settings',
        meta: '⌘,',
        icon: GEAR,
        go: () => openPrefs('general'),
      },
    ],
    [show, openPrefs],
  )

  /* Starting an agent is opening a terminal and typing its name into it —
     which is what a person would do, and is why the agent belongs to the
     terminal that runs it. Nothing is spawned beside the pane: a process the
     terminal does not own is one that closing the tab cannot stop.

     The ones that are not installed stay on the list and say so. A short list
     with no explanation is the same screen as a broken menu. */
  const agentRows: Row[] = useMemo(
    () =>
      agents.map((agent) => ({
        key: `agent:${agent.id}`,
        name: agent.label,
        meta: agent.installed ? agent.launch : 'not installed',
        icon: <AgentMark agent={agent.id} />,
        go: () => {
          if (!agent.installed) return
          show('term', { title: agent.label, launch: agent.id })
        },
      })),
    [agents, show],
  )

  /* A session is a terminal or a chat that is open; focusing it is the whole
     act, because it is already there. */
  const sessions: Row[] = useMemo(
    () =>
      tabs
        .filter((tab) => tab.kind === 'term' || tab.kind === 'chat')
        .map((tab) => ({
          key: `tab:${tab.id}`,
          name: tab.title ?? (tab.kind === 'term' ? 'Terminal' : 'Chat'),
          meta: tab.kind,
          icon: PANES.find((pane) => pane.name === tab.kind)!.icon,
          go: () => focus(tab.id),
        })),
    [tabs, focus],
  )


  return { panes, agents: agentRows, sessions, cards, files, indexing, partial }
}
