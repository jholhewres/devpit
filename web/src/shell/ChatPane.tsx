import { useEffect, useRef, useState } from 'react'

import { Asked } from './Asked'
import { MODES, money, ready } from './chat'
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
          {chat.messages.length === 0 && !chat.error && (
            <div className="exempty__t">Nothing said yet.</div>
          )}
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
            <Account chat={chat} />
            <Model chat={chat} />
            <select
              className="chip"
              value={chat.permission}
              onChange={(event) => chat.setPermission(event.target.value)}
              aria-label="Permission"
            >
              {MODES.map((mode) => (
                <option key={mode.id} value={mode.id}>
                  {mode.label}
                </option>
              ))}
            </select>
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

/* The account, once and for good. After the first turn it reads rather than
   asks: one transcript belongs to one account. */
function Account({ chat }: { chat: ReturnType<typeof useChat> }): React.JSX.Element {
  if (chat.fixed) {
    const label = chat.profiles.find((profile) => profile.id === chat.fixed)?.label ?? chat.fixed
    return (
      <span className="chip" title="A conversation keeps the account it started on">
        {label}
      </span>
    )
  }
  if (chat.profiles.length === 0) {
    return <span className="chip">No agent CLI on the PATH</span>
  }
  return (
    <select
      className="chip"
      value={chat.profileId ?? ''}
      onChange={(event) => chat.pick(event.target.value)}
      aria-label="Account"
    >
      <option value="" disabled>
        Account
      </option>
      {chat.profiles.map((profile) => (
        <option key={profile.id} value={profile.id}>
          {profile.label}
        </option>
      ))}
    </select>
  )
}

function Model({ chat }: { chat: ReturnType<typeof useChat> }): React.JSX.Element | null {
  const models = chat.profiles.find((profile) => profile.id === chat.profileId)?.models ?? []
  if (models.length === 0) return null
  return (
    <select
      className="chip"
      value={chat.model ?? models[0]}
      onChange={(event) => chat.setModel(event.target.value)}
      aria-label="Model"
    >
      {models.map((model) => (
        <option key={model} value={model}>
          {model}
        </option>
      ))}
    </select>
  )
}
