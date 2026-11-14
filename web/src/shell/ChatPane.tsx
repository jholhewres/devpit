import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { Context } from '../gen/bindings'

import { ActivityContext, activityOf, keepActivity, seen, type Activity } from './activity'
import { ActivityPick } from './ActivityPick'
import { AssistantToggle } from './AssistantToggle'
import { ASKS, CONTINUE, money, putBackFrom } from './chat'
import type { Extras } from './queueing'
import { titleOf, type Tab } from './strip'
import { ChatCardChip } from './ChatCardChip'
import { ChatFind } from './ChatFind'
import { useMessagesPick } from './chatMessages'
import { ChatThread } from './ChatThread'
import { Composer, type ComposerHandle } from './Composer'
import { ContextMeter } from './ContextMeter'
import { CopySession } from './CopySession'
import { DocPeek } from './DocPeek'
import { AppsScopeContext } from './mcpApps'
import { WholeFrom } from './WholeFrom'
import { DropTarget } from './DropTarget'
import { EditFrom } from './EditFrom'
import { PaneCorner } from './PaneCorner'
import { PromptRail } from './PromptRail'
import { Quoting } from './Quoting'
import { useTaking } from './useTaking'
import { useThread } from './useOlder'
import { ChatBlank } from './ChatBlank'
import { RemoteToggle } from './RemoteToggle'
import { SessionsChip } from './OrchestratorChat'
import { SessionCost } from './SessionCost'
import { useChat, type ChatCard } from './useChat'
import { useJump, type Reveal } from './useJump'
import { useShell } from './useShell'
import { useShortcut } from './useShortcut'

/* A message sent again goes as it was written: its files are already named in its words. */
const NONE: Extras = { files: [], skills: [] }

/*
 * One conversation, per tab.
 *
 * The tab id is the conversation id: opening a second chat is a second
 * conversation, not a second view of the first.
 *
 * The thread and the composer are two centred columns rather than one
 * full-width sheet: a line of prose across a wide monitor is a line nobody
 * finishes, and the composer reads as a thing you write in rather than a bar
 * bolted to the bottom of the window.
 */

export function ChatPane({ tab }: { tab: Tab }): React.JSX.Element {
  const { active, project, rename, drafted, finished } = useShell()
  const chat = useChat(tab.id)
  const { pasted, dropped } = useTaking(chat)
  const follow = useThread(project?.id ?? null, tab.id, chat)
  const composer = useRef<ComposerHandle>(null)

  const mine = active?.id === tab.id
  const where = useMemo(() => (project ? { projectId: project.id, conversationId: tab.id } : null), [project, tab.id])
  const reveal = useRef<Reveal | null>(null)
  const jump = useJump(where, chat, follow.hold, reveal)
  /* What the pane reads of the messages: none of it moves with a streamed batch. */
  const { count, said, asked } = useMessagesPick(chat.messages, (all) => ({
    count: all.length,
    said: all.find((one) => one.role === 'user'),
    asked: all.reduce((sum, one) => sum + (one.role === 'user' ? 1 : 0), 0),
  }))
  const [finding, setFinding] = useState(false)
  const [activity, setActivity] = useState(() => activityOf(tab.id))
  const pickActivity = useCallback((picked: Activity) => {
    keepActivity(tab.id, picked)
    setActivity(picked)
  }, [tab.id])
  useShortcut('findChat', () => {
    if (!mine || count === 0) return false
    setFinding(true)
  })
  /* A file an answer names opens beside the conversation, not over it. */
  const [peek, setPeek] = useState<string | null>(null)
  /* An edit waiting on whether the files changed since go back too. */
  const [editing, setEditing] = useState<{ turnId: string; files: number } | null>(null)
  const { messages, editFrom } = chat
  const edit = useCallback((turnId: string) => {
    const files = putBackFrom(messages.get(), turnId)
    if (files > 0) setEditing({ turnId, files })
    else editFrom(turnId, false)
  }, [messages, editFrom])
  /* Where a page an MCP tool came with is asked for: this conversation's account, in its folder. */
  const scope = useMemo(
    () => (chat.profileId && chat.folder ? { profileId: chat.profileId, cwd: chat.folder } : null),
    [chat.profileId, chat.folder],
  )

  useEffect(() => {
    if (tab.draft !== undefined) drafted(tab.id)
  }, [tab.draft, tab.id, drafted])

  /* Named once after the first thing you said in it; a tab you renamed keeps your name. */
  useEffect(() => {
    if (tab.title !== undefined || !said) return
    const name = titleOf(said.parts.map((part) => ('text' in part ? part.text : '')).join(''))
    if (name) rename(tab.id, name)
  }, [rename, said, tab.id, tab.title])

  /* A turn that ends while this tab is behind another marks the tab. */
  const wasSending = useRef(chat.sending)
  useEffect(() => {
    if (wasSending.current && !chat.sending) finished(tab.id)
    wasSending.current = chat.sending
  }, [chat.sending, finished, tab.id])

  const empty = count === 0 && !chat.error

  return (
    <>
      <ChatCorner tabId={tab.id} card={chat.card} context={chat.context} session={chat.session} cost={chat.cost} orchestrator={project?.orchestrator ?? null} projectId={project?.id ?? null} sending={chat.sending} supervised={ASKS(chat.permission)} activity={activity} onActivity={pickActivity} />

      <DropTarget mine={mine} onDrop={dropped} />
      <div className="scroll" ref={follow.box}>
        {empty ? (
          <ChatBlank project={project} account={chat.profiles.find((one) => one.id === project?.orchestrator)?.label} onTry={(text) => composer.current?.put(text)} />
        ) : (
          <AppsScopeContext.Provider value={scope}>
            <ActivityContext.Provider value={seen(activity, finding)}>
            <WholeFrom conversationId={tab.id}>
              <ChatThread messages={chat.messages} box={follow.box} top={follow.top} older={chat.older} stalled={follow.stalled} onMore={follow.more} sending={chat.sending} error={chat.error} folder={chat.folder ?? null} rewindable={chat.rewindable} onRewind={chat.rewind} onEdit={edit} onOpen={setPeek} onContinue={() => chat.say(CONTINUE)} onRetry={(text) => chat.say(text, undefined, NONE)} onResend={(text) => (chat.sending ? void chat.steer(text, NONE) : chat.say(text, undefined, NONE))} reveal={reveal} />
            </WholeFrom>
            </ActivityContext.Provider>
          </AppsScopeContext.Provider>
        )}
      </div>

      {!empty && <PromptRail where={where} said={`${asked}:${chat.sending}`} jump={jump} />}
      {finding && <ChatFind where={where} box={follow.box} jump={jump} onDone={() => setFinding(false)} />}
      <Quoting box={follow.box} onQuote={(text) => composer.current?.add(text)} />
      {editing && <EditFrom files={editing.files} onCancel={() => setEditing(null)} onPick={(putBack) => (chat.editFrom(editing.turnId, putBack), setEditing(null))} />}

      <Composer tab={tab} chat={chat} project={project ?? null} mine={mine} away={follow.away} toEnd={follow.toEnd} handle={composer} pasted={pasted} />
      {peek && <DocPeek path={peek} onOpen={setPeek} onClose={() => setPeek(null)} />}
    </>
  )
}

/* The pane's corner, drawn again only when what it shows changed. */
const ChatCorner = memo(function ChatCorner({
  tabId,
  card,
  context,
  session,
  cost,
  orchestrator,
  projectId,
  sending,
  supervised,
  activity,
  onActivity,
}: {
  tabId: string
  card: ChatCard | null
  context: Context | null
  session: string | null
  cost: number
  orchestrator: string | null
  projectId: string | null
  sending: boolean
  supervised: boolean
  activity: Activity
  onActivity: (activity: Activity) => void
}): React.JSX.Element {
  return (
    <PaneCorner tabId={tabId} what="chat">
      <ActivityPick value={activity} onPick={onActivity} />
      <ChatCardChip card={card} />
      <ContextMeter context={context} />
      {session ? <SessionCost sessionId={session} className="pcorner__cost scost" /> : money(cost) && <span className="pcorner__cost">{money(cost)}</span>}
      {session && <CopySession id={session} />}
      {orchestrator && <SessionsChip profileId={orchestrator} />}
      {orchestrator && projectId && <AssistantToggle projectId={projectId} />}
      {projectId && <RemoteToggle projectId={projectId} conversationId={tabId} sending={sending} supervised={supervised} />}
    </PaneCorner>
  )
})
