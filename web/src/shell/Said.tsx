import type { Message } from '../gen/bindings'
import { isLong, sentWith, type SentFile } from './sentFiles'
import { useShellPick } from './shellStore'
import { useStream } from './Streamed'
import { foldOf, useDisclosed } from './useDisclosed'

/* What a person said: the files as chips and pictures, then the words.
   `onResend` when it was steered into a turn that ended without taking it in. */
export function Said({ message, onResend }: { message: Message; onResend?: () => void }): React.JSX.Element {
  const { files, prompt } = sentWith(message.parts.map((part) => ('text' in part ? part.text : '')).join(''))
  const long = isLong(prompt)
  const projectId = useShellPick((shell) => shell.project?.id ?? null)
  const [open, setOpen] = useDisclosed(foldOf(message.id, 'said'), false)

  return (
    <article className="said" data-lost={onResend ? '' : undefined}>
      <div className="said__b">
        {files.length > 0 && (
          <div className="said__files">
            {files.map((file) => (file.image ? <Picture key={file.path} file={file} projectId={projectId} /> : <Chip key={file.path} file={file} />))}
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
      {onResend && (
        <p className="said__lost">
          Didn't reach the agent.
          <button onClick={onResend}>Resend</button>
        </p>
      )}
    </article>
  )
}

const Chip = ({ file }: { file: SentFile }): React.JSX.Element => (
  <span className="said__chip" title={file.path}>
    <span className="said__k">{file.kind || 'file'}</span>
    {file.name}
  </span>
)

/* Streamed by devpit's media scheme: a paste lives outside the project, where the page cannot reach. */
function Picture({ file, projectId }: { file: SentFile; projectId: string | null }): React.JSX.Element {
  const { url, error } = useStream(projectId, file.path)
  if (error) return <Chip file={file} />
  return (
    <span className="said__pic" title={file.path}>
      {url && <img src={url} alt={file.name} />}
    </span>
  )
}
