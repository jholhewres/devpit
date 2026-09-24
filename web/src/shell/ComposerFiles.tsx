import type { Chat } from './useChat'

/* What is attached to the next message, each one a click away from gone. */
export function ComposerFiles({ chat }: { chat: Chat }): React.JSX.Element | null {
  if (chat.files.length === 0) return null
  return (
    <div className="composer__files">
      {chat.files.map((file) => (
        <button key={file.path} className="chip" onClick={() => chat.detach(file.path)} title={`${file.path} — click to remove`}>
          {chat.previews[file.path] && <img className="chip__thumb" src={chat.previews[file.path]} alt="" />}
          {file.name} ✕
        </button>
      ))}
    </div>
  )
}
