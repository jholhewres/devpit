import { useEffect, useRef, useState } from 'react'

import { Asked } from './Asked'
import { Threads } from './Threads'
import { Chips } from './Chips'
import { money, ready } from './chat'
import type { Tab } from './strip'
import { Turn } from './Turn'
import { useChat } from './useChat'
import { useShell } from './useShell'
import { onFilesDropped } from './window'

/*
 * One conversation, per tab.
 *
 * The tab id is the conversation id: opening a second chat is a second
 * conversation, not a second view of the first.
 */

export function ChatPane({ tab }: { tab: Tab }): React.JSX.Element {
  const { close, active } = useShell()
  const chat = useChat(tab.id)
  const [prompt, setPrompt] = useState('')
  const box = useRef<HTMLDivElement>(null)

  /* A drop lands on the window, not on a pane, so only the chat in front
     takes it. */
  const mine = active?.id === tab.id
  useEffect(() => {
    if (!mine) return
    return onFilesDropped(chat.attach)
  }, [mine, chat.attach])

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

  const picked = chat.profiles.find((profile) => profile.id === chat.profileId)
  const spent = money(chat.cost)

  return (
    <>
      <div className="pane__bar">
        <span className="pane__t">
          <b>Chat</b>
          {picked ? ` · ${picked.label}` : ''}
          {spent ? ` · ${spent}` : ''}
        </span>
        <span className="drag"></span>
        <button className="sq26" onClick={() => close(tab.id)} aria-label="Close chat">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div className="scroll" ref={box}>
        <div className="thread">
          {chat.error && <div className="exempty__t">{chat.error}</div>}
          {chat.messages.length === 0 && !chat.error && <Threads hide={tab.id} />}
          {chat.messages.map((message) => (
            <Turn key={message.id} message={message} />
          ))}

          <Asked questions={chat.asked} onAnswer={chat.answer} />
        </div>
      </div>

      <div className="composer">
        <div className="composer__in">
          <textarea
            className="composer__ph"
            placeholder="Do anything…"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            onKeyDown={(event) => {
              /* Enter sends; Shift+Enter is a new line, as everywhere else. */
              if (event.key === 'Enter' && !event.shiftKey) {
                event.preventDefault()
                send()
              }
            }}
          />
          {chat.files.length > 0 && (
            <div className="composer__row">
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
          <div className="composer__row">
            <Chips chat={chat} />
            {chat.sending ? (
              <button className="send" onClick={chat.stop} aria-label="Stop">
                <svg width="11" height="11" viewBox="0 0 24 24" fill="currentColor"><rect x="5" y="5" width="14" height="14" rx="2" /></svg>
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
      </div>
    </>
  )
}
