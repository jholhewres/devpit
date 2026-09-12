import { useEffect, useRef, useState } from 'react'

import { Asked } from './Asked'
import { Chips } from './Chips'
import { money, ready } from './chat'
import { targetOf } from './stop'
import { titleOf, type Tab } from './strip'
import { Turn } from './Turn'
import { useChat } from './useChat'
import { useShell } from './useShell'
import { useStop } from './useStop'
import { onFilesDropped } from './window'

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
  const { close, active, project, rename } = useShell()
  const chat = useChat(tab.id)
  const [prompt, setPrompt] = useState('')
  const box = useRef<HTMLDivElement>(null)
  const field = useRef<HTMLTextAreaElement>(null)

  /* A drop lands on the window, not on a pane, so only the chat in front
     takes it. */
  const mine = active?.id === tab.id
  useEffect(() => {
    if (!mine) return
    return onFilesDropped(chat.attach)
  }, [mine, chat.attach])

  /* A conversation is called the first thing you said in it. Only once, and
     only while it is unnamed: a tab you renamed keeps the name you gave it. */
  const said = chat.messages.find((one) => one.role === 'user')
  useEffect(() => {
    if (tab.title !== undefined || !said) return
    const name = titleOf(said.parts.map((part) => ('text' in part ? part.text : '')).join(''))
    if (name) rename(tab.id, name)
  }, [rename, said, tab.id, tab.title])

  /* New output belongs at the bottom, where the eye already is. */
  useEffect(() => {
    const scroll = box.current
    if (scroll) scroll.scrollTop = scroll.scrollHeight
  }, [chat.messages])

  const send = (): void => {
    if (!ready(prompt, chat.sending, chat.profileId)) return
    chat.say(prompt)
    setPrompt('')
  }

  /* The turn in flight is the one still streaming; arming is tied to it so a
     press that armed against a finished turn cannot stop the next one. */
  const showEsc = useStop(
    chat.sending && mine,
    targetOf(tab.id, chat.messages.find((one) => one.streaming)?.turnId ?? null),
    chat.stop,
  )

  const spent = money(chat.cost)
  const empty = chat.messages.length === 0 && !chat.error

  return (
    <>
      <div className="pane__bar">
        <span className="pane__t">
          <b>Chat</b>
          {spent ? ` · ${spent}` : ''}
        </span>
        <span className="drag"></span>
        <button className="sq26" onClick={() => close(tab.id)} aria-label="Close chat">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

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
              <Turn key={message.id} message={message} />
            ))}
          </div>
        )}
      </div>

      <div className="composer">
        <div className="composer__col">
          {/* The question sits above the composer, where your hands are — not
              in the thread, where it scrolls away from you. */}
          <Asked questions={chat.asked} onAnswer={chat.answer} />

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
                    {file.name} ✕
                  </button>
                ))}
              </div>
            )}

            <textarea
              className="composer__ph"
              placeholder="Do anything…"
              ref={field}
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              onKeyDown={(event) => {
                /* Enter sends; Shift+Enter is a new line, as everywhere. */
                if (event.key === 'Enter' && !event.shiftKey) {
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
              {chat.sending ? (
                <button
                  className="send"
                  onClick={chat.stop}
                  aria-label={showEsc ? 'Press Escape again to stop' : 'Stop'}
                >
                  {showEsc ? (
                    <span className="send__esc">Esc</span>
                  ) : (
                    <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor"><rect x="5" y="5" width="14" height="14" rx="2" /></svg>
                  )}
                </button>
              ) : (
                <button
                  className="send"
                  onClick={send}
                  disabled={!ready(prompt, chat.sending, chat.profileId)}
                  aria-label="Send"
                >
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 19V5M5 12l7-7 7 7" /></svg>
                </button>
              )}
            </div>
          </div>

          {/* Where the work happens. Quieter than the composer's own chips,
              because these describe the conversation rather than steer the
              turn. */}
          <div className="composer__where">
            <span className="wschip">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg>
              {project?.name ?? 'No project'}
            </span>
            <span className="wschip">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" /><path d="M8 21h8M12 17v4" /></svg>
              Local
            </span>
            {project?.worktrees[0] && (
              <span className="wschip">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><line x1="6" y1="3" x2="6" y2="15" /><circle cx="18" cy="6" r="3" /><circle cx="6" cy="18" r="3" /><path d="M18 9a9 9 0 0 1-9 9" /></svg>
                {project.worktrees[0].branch}
              </span>
            )}
          </div>
        </div>
      </div>
    </>
  )
}
