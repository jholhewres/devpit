import type { LiveSession } from '../gen/bindings'
import { ask, commands } from './live'
import { refreshSessions, useLiveSessions } from './liveStatus'

/*
 * "Send it", said in the orchestrator's chat while one draft waits, sends that
 * draft as the person — the click on its card, in words. Only a bare "send":
 * anything more is a message for the orchestrator.
 */

const SEND = /^(send( it)?|go ahead|envie|envia|manda|mande|pode enviar)[.!]*$/i

/** The one draft a bare "send" sends, or null when there is none or more than one. */
export function draftToSend(text: string, sessions: readonly LiveSession[]): (LiveSession & { draft: string }) | null {
  if (!SEND.test(text.trim())) return null
  const drafted = sessions.filter((one): one is LiveSession & { draft: string } => Boolean(one.draft) && one.waiting === null)
  return drafted.length === 1 ? drafted[0] : null
}

/** Sends the draft `text` asks for; answers whether it took the message. */
export function useDraftShortcut(profileId: string | null, putBack: (text: string) => void): (text: string) => boolean {
  const live = useLiveSessions(profileId)
  return (text) => {
    const one = profileId ? draftToSend(text, live) : null
    if (!one || !profileId) return false
    void ask(() => commands.orchestratorReply(profileId, one.name, one.draft, 'the chat')).then((sent) => {
      if (sent.error) putBack(text)
      refreshSessions(profileId)
    })
    return true
  }
}
