import { useEffect, useMemo, useRef, useState } from 'react'

import { ASKS, CONTINUE, money } from './chat'
import { titleOf, type Tab } from './strip'
import { ChatCardChip } from './ChatCardChip'
import { ChatFind } from './ChatFind'
import { ChatThread } from './ChatThread'
import { Composer, type ComposerHandle } from './Composer'
import { ContextMeter } from './ContextMeter'
import { CopySession } from './CopySession'
import { DocPeek } from './DocPeek'
import { AppsScopeContext } from './mcpApps'
import { WholeFrom } from './WholeFrom'
import { DropTarget } from './DropTarget'
import { PaneCorner } from './PaneCorner'
import { PromptRail } from './PromptRail'
import { useTaking } from './useTaking'
import { useThread } from './useOlder'
import { ChatBlank } from './ChatBlank'
import { RemoteToggle } from './RemoteToggle'
import { SessionsChip } from './OrchestratorChat'
import { SessionCost } from './SessionCost'
import { useChat } from './useChat'
import { useJump, type Reveal } from './useJump'
import { useShell } from './useShell'
import { useShortcut } from './useShortcut'

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
  const [finding, setFinding] = useState(false)
  useShortcut('findChat', () => {
    if (!mine || chat.messages.length === 0) return false
    setFinding(true)
  })
  /* A file an answer names opens beside the conversation, not over it. */
  const [peek, setPeek] = useState<string | null>(null)
  /* Where a page an MCP tool came with is asked for: this conversation's account, in its folder. */
  const scope = useMemo(
    () => (chat.profileId && chat.folder ? { profileId: chat.profileId, cwd: chat.folder } : null),
    [chat.profileId, chat.folder],
  )

  useEffect(() => {
    if (tab.draft !== undefined) drafted(tab.id)
  }, [tab.draft, tab.id, drafted])

  /* Named once after the first thing you said in it; a tab you renamed keeps your name. */
  const said = chat.messages.find((one) => one.role === 'user')
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

  const empty = chat.messages.length === 0 && !chat.error

  return (
    <>
      <PaneCorner tabId={tab.id} what="chat">
        <ChatCardChip card={chat.card} />
        <ContextMeter context={chat.context} />
        {chat.session ? <SessionCost sessionId={chat.session} className="pcorner__cost scost" /> : money(chat.cost) && <span className="pcorner__cost">{money(chat.cost)}</span>}
        {chat.session && <CopySession id={chat.session} />}
        {project?.orchestrator && <SessionsChip profileId={project.orchestrator} />}
        {project && <RemoteToggle projectId={project.id} conversationId={tab.id} sending={chat.sending} supervised={ASKS(chat.permission)} />}
      </PaneCorner>

      <DropTarget mine={mine} onDrop={dropped} />
      <div className="scroll" ref={follow.box}>
        {empty ? (
          <ChatBlank project={project} account={chat.profiles.find((one) => one.id === project?.orchestrator)?.label} onTry={(text) => composer.current?.put(text)} />
        ) : (
          <AppsScopeContext.Provider value={scope}>
            <WholeFrom conversationId={tab.id}>
              <ChatThread messages={chat.messages} box={follow.box} top={follow.top} older={chat.older} stalled={follow.stalled} onMore={follow.more} sending={chat.sending} error={chat.error} folder={chat.folder ?? null} rewindable={chat.rewindable} onRewind={chat.rewind} onOpen={setPeek} onContinue={() => chat.say(CONTINUE)} reveal={reveal} />
            </WholeFrom>
          </AppsScopeContext.Provider>
        )}
      </div>

      {!empty && <PromptRail where={where} said={`${chat.messages.filter((one) => one.role === 'user').length}:${chat.sending}`} jump={jump} />}
      {finding && <ChatFind where={where} box={follow.box} jump={jump} onDone={() => setFinding(false)} />}

      <Composer tab={tab} chat={chat} project={project ?? null} mine={mine} away={follow.away} toEnd={follow.toEnd} handle={composer} pasted={pasted} />
      {peek && <DocPeek path={peek} onOpen={setPeek} onClose={() => setPeek(null)} />}
    </>
  )
}
