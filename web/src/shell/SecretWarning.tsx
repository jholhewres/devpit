import { looksSecret } from './secretShape'

/* Said while it can still be taken out: sent, a key stays in the transcript. */
export function SecretWarning({ text }: { text: string }): React.JSX.Element | null {
  if (!looksSecret(text)) return null
  return (
    <p className="composer__warn" role="status">
      This looks like a key. Sent, it stays in the conversation&rsquo;s transcript &mdash; keep it in the project&rsquo;s secrets instead (Edit project &rarr; Secrets), and the session reads it by its name.
    </p>
  )
}
