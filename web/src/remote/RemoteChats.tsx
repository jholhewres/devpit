import { useEffect } from 'react'

import type { Conversation, Conversations, RemoteIn } from '../gen/bindings'

/* The project's chats, read: each one's words, latest last. */

export function RemoteChats({
  project,
  chats,
  chat,
  onClose,
  send,
}: {
  project: string
  chats: Conversations | null
  chat: Conversation | null
  onClose: () => void
  send: (message: RemoteIn) => void
}): React.JSX.Element {
  useEffect(() => {
    if (!chat) return
    const again = window.setInterval(() => send({ t: 'chat', project, conversation: chat.id }), 8000)
    return () => window.clearInterval(again)
  }, [chat, project, send])

  if (chat)
    return (
      <div className="rm__chat">
        <button className="rm__btn" onClick={onClose}>
          ← Chats
        </button>
        {chat.messages.map((message) => (
          <div className="rm__msg" key={message.id} data-role={message.role}>
            {message.parts.map((part, at) => (part.kind === 'text' ? <p key={at}>{part.text}</p> : null))}
          </div>
        ))}
      </div>
    )
  if (!chats) return <p className="rm__said">Reading the chats…</p>
  if (chats.conversations.length === 0) return <p className="rm__said">No chats in this project.</p>
  return (
    <div className="rm__list">
      {chats.conversations.map((one) => (
        <button key={one.id} className="rm__row" onClick={() => send({ t: 'chat', project, conversation: one.id })}>
          <span>{one.title || 'Untitled'}</span>
        </button>
      ))}
    </div>
  )
}
