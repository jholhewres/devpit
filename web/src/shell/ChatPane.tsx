import { useEffect, useRef, useState } from 'react'

import { Asked } from './Asked'
import { Chips } from './Chips'
import { money, ready, unanswered } from './chat'
import { targetOf } from './stop'
import { titleOf, type Tab } from './strip'
import { ChatCardChip } from './ChatCardChip'
import { ChatWhere } from './ChatWhere'
import { ContextMeter } from './ContextMeter'
import { ComposerStatus } from './ComposerStatus'
import { CopySession } from './CopySession'
import { DropTarget } from './DropTarget'
import { PaneCorner } from './PaneCorner'
import { useTaking } from './useTaking'
import { Turn } from './Turn'
import { SkillPills } from './SkillPills'
import { ResumePicker } from './ResumePicker'
import { useFollow } from './useFollow'
import { Queued } from './Queued'
import { SendButton } from './SendButton'
import { useQueue } from './useQueue'
import { SlashMenu } from './SlashMenu'
import { MentionMenu } from './MentionMenu'
import { useMention } from './useMention'
import { useChat } from './useChat'
import { useSlash } from './useSlash'
import { useShell } from './useShell'
import { useStop } from './useStop'
import { committed } from './typing'

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
  const { active, project, rename, drafted } = useShell()
  const chat = useChat(tab.id)
  const { pasted, dropped } = useTaking(chat)
  /* A chat opened from a card starts with the card in the composer, unsent. */
  const [prompt, setPrompt] = useState(tab.draft ?? '')
  const slash = useSlash(chat.profileId, prompt, setPrompt)
  /* New output belongs at the bottom, where the eye already is — unless the
     eye went up to read. */
  const box = useFollow<HTMLDivElement>(chat.messages)
  const field = useRef<HTMLTextAreaElement>(null)
  const mention = useMention(project?.id ?? null, prompt, field, setPrompt)

  const mine = active?.id === tab.id

  useEffect(() => {
    if (tab.draft !== undefined) drafted(tab.id)
  }, [tab.draft, tab.id, drafted])

  /* A conversation is called the first thing you said in it. Only once, and
     only while it is unnamed: a tab you renamed keeps the name you gave it. */
  const said = chat.messages.find((one) => one.role === 'user')
  useEffect(() => {
    if (tab.title !== undefined || !said) return
    const name = titleOf(said.parts.map((part) => ('text' in part ? part.text : '')).join(''))
    if (name) rename(tab.id, name)
  }, [rename, said, tab.id, tab.title])

  /* Typed while a turn runs, a message waits for it rather than being refused. */
  const queue = useQueue(chat.sending, chat.say)
  const send = (): void => {
    if (!ready(prompt, false, chat.profileId)) return
    if (chat.sending) queue.add(prompt)
    else chat.say(prompt)
    setPrompt('')
    box.current?.scrollTo({ top: box.current.scrollHeight })
  }

  /* A stop hands what was queued back to the composer: it was meant for a
     turn that is no longer going to happen as planned. */
  const halt = (): void => {
    const back = queue.takeAll()
    if (back) setPrompt((was) => [back, was].filter(Boolean).join('\n\n'))
    chat.stop()
  }

  /* The turn in flight is the one still streaming; arming is tied to it so a
     press that armed against a finished turn cannot stop the next one. */
  const showEsc = useStop(
    chat.sending && mine,
    targetOf(tab.id, chat.messages.find((one) => one.streaming)?.turnId ?? null),
    halt,
  )

  const spent = money(chat.cost)
  const empty = chat.messages.length === 0 && !chat.error

  return (
    <>
      <PaneCorner tabId={tab.id} what="chat">
        <ChatCardChip card={chat.card} />
        <ContextMeter context={chat.context} />
        {spent && <span className="pcorner__cost" title="What this conversation has cost">{spent}</span>}
        {chat.session && <CopySession id={chat.session} />}
      </PaneCorner>

      <DropTarget mine={mine} onDrop={dropped} />
      <div className="scroll" ref={box}>
        {empty ? (
          <div className="chat__blank">
            <div className="chat__ask">
              <svg className="chat__mark" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4" /></svg>
              <h2 className="chat__q">
                What should we build{project ? ' in ' : ''}
                {project && <span className="chat__where">{project.name}</span>}?
              </h2>
            </div>
          </div>
        ) : (
          <div className="thread">
            {chat.error && <div className="exempty__t">{chat.error}</div>}
            {chat.messages.map((message) => (
              <Turn key={message.id} message={message} onRewind={message.turnId && chat.rewindable.includes(message.turnId) ? chat.rewind : undefined} />
            ))}
            {unanswered(chat.messages, chat.sending) && <p className="said__cmd">No answer was saved for this — the app closed while the turn was running.</p>}
          </div>
        )}
      </div>

      <div className="composer">
        <div className="composer__col">
          {/* The question sits above the composer, where your hands are — not
              in the thread, where it scrolls away from you. */}
          <Asked questions={chat.asked} onAnswer={chat.answer} />
          <ComposerStatus conversationId={tab.id} messages={chat.messages} />
          <Queued queue={queue} />

          <div className="composer__in">
            {chat.files.length > 0 && (
              <div className="composer__files">
                {chat.files.map((file) => (
                  <button
                    key={file.path}
                    className="chip"
                    onClick={() => chat.detach(file.path)}
                    title={`${file.path} — click to remove`}
                  >
                    {chat.previews[file.path] && (
                      <img className="chip__thumb" src={chat.previews[file.path]} alt="" />
                    )}
                    {file.name} ✕
                  </button>
                ))}
              </div>
            )}

            <SkillPills picked={chat.skills} onChange={chat.setSkills} profile={chat.profiles.find((one) => one.id === chat.profileId)} />
            <SlashMenu slash={slash} />
            <MentionMenu menu={mention} />
            {slash.own === 'resume' && <ResumePicker from={tab.id} profileId={chat.profileId} onClose={() => (slash.closeOwn(), field.current?.focus())} />}
            <textarea
              className="composer__ph"
              placeholder="Do anything… @ for a file, / for a command"
              ref={field}
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              onPaste={pasted}
              onKeyDown={(event) => {
                /* Enter sends, Shift+Enter is a new line — and `committed`
                   keeps the Enter that finishes an accented letter out. */
                if (slash.keyDown(event) || mention.keyDown(event)) return
                if (committed(event) && !event.shiftKey) {
                  event.preventDefault()
                  send()
                }
              }}
            />

            <div className="composer__row">
              {/* A control takes focus to be operated, and gives it back when
                  it is done: the cursor was mid-sentence, and putting it back
                  is not the person's job. Cancelling the press instead would
                  be cheaper and would swallow the click that opens the menu. */}
              <Chips chat={chat} refocus={() => field.current?.focus()} />
              <SendButton sending={chat.sending} showEsc={showEsc} can={ready(prompt, chat.sending, chat.profileId)} onSend={send} onStop={halt} />
            </div>
          </div>

          <ChatWhere />
        </div>
      </div>
    </>
  )
}
