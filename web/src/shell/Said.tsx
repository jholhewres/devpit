import { useContext, useEffect, useState } from 'react'

import { WholeContext } from './cutOutput'
import { ask, commands } from './live'
import type { Message } from '../gen/bindings'
import { isLong, sentWith, type SentFile } from './said'
import { useDisclosed } from './useDisclosed'

/* What a person said: the files as chips and pictures, then the words. */
export function Said({ message }: { message: Message }): React.JSX.Element {
  const { files, prompt } = sentWith(message.parts.map((part) => ('text' in part ? part.text : '')).join(''))
  const long = isLong(prompt)
  const [open, setOpen] = useDisclosed(`${message.id}:said`, false)

  return (
    <article className="said">
      <div className="said__b">
        {files.length > 0 && (
          <div className="said__files">
            {files.map((file) => (file.image ? <Picture key={file.path} file={file} /> : <Chip key={file.path} file={file} />))}
          </div>
        )}
        {prompt && (
          <div className="said__t" data-folded={(long && !open) || undefined}>
            {prompt}
          </div>
        )}
        {long && (
          <button className="said__more" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
            {open ? 'Show less' : 'Show more'}
          </button>
        )}
      </div>
    </article>
  )
}

const Chip = ({ file }: { file: SentFile }): React.JSX.Element => (
  <span className="said__chip" title={file.path}>
    <span className="said__k">{file.kind || 'file'}</span>
    {file.name}
  </span>
)

/* Read through the backend: a paste lives outside the project, where the page cannot reach. */
const seen = new Map<string, string>()

function Picture({ file }: { file: SentFile }): React.JSX.Element {
  const where = useContext(WholeContext)
  const key = `${where?.projectId ?? ''}:${file.path}`
  const [src, setSrc] = useState<string | null>(() => seen.get(key) ?? null)
  const [lost, setLost] = useState(false)

  useEffect(() => {
    if (src || !where) return
    let on = true
    void ask(() => commands.chatImage(where.projectId, file.path)).then((read) => {
      if (!on) return
      if (read.data) {
        if (seen.size > 64) seen.delete(seen.keys().next().value!)
        seen.set(key, read.data)
        setSrc(read.data)
      } else setLost(true)
    })
    return () => {
      on = false
    }
  }, [src, where, file.path, key])

  if (lost || !where) return <Chip file={file} />
  return (
    <span className="said__pic" title={file.path}>
      {src && <img src={src} alt={file.name} />}
    </span>
  )
}
