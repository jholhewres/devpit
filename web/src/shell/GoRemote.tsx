import { useShell } from './useShell'

/* An orchestrator's conversation, continued in a terminal where Remote
   Control can reach it from a phone or claude.ai. The chat lets its own
   process go for it; coming back is closing the terminal and writing here. */
export function GoRemote({ conversationId, account, name }: { conversationId: string; account: string; name: string }): React.JSX.Element {
  const { show } = useShell()
  return (
    <button
      className="sq26"
      title="Continue this conversation remotely — from your phone or claude.ai"
      aria-label="Continue remotely"
      onClick={() => show('term', { title: `Remote · ${name}`, launch: account, resume: conversationId })}
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="7" y="2" width="10" height="20" rx="2" /><path d="M11 18h2" /></svg>
    </button>
  )
}
