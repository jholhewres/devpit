import type { LivePane, Message, RemoteDraft, RemoteIn, RemoteOut } from '../gen/bindings'
import { applied } from '../shell/frames'
import { KEPT_BYTES, keptAsPicture } from '../shell/pasting'

/*
 * Talking from the phone: who the chat view shows, what the machine's words
 * make of it, and the pieces a photo or a recording goes to the machine in.
 */

/** An orchestrator's conversation, any project's chat, or a session. */
export type Talking =
  /** `orchestrator` is the account an orchestrator's chat speaks as. */
  | { kind: 'chat'; project: string; conversation: string; title: string; orchestrator: string | null }
  | { kind: 'session'; profile: string; name: string; title: string; project: string | null; pane: LivePane | null }

/** The replies drafted for the person that belong with what is shown. */
export function draftsOf(talking: Talking, drafts: readonly RemoteDraft[]): readonly RemoteDraft[] {
  return drafts.filter((one) =>
    talking.kind === 'chat' ? one.profile === talking.orchestrator : one.profile === talking.profile && one.name === talking.name,
  )
}

/** What a conversation or a session is asked for, to be read. */
export function asked(talking: Talking): RemoteIn {
  return talking.kind === 'chat'
    ? { t: 'chat', project: talking.project, conversation: talking.conversation }
    : { t: 'session', profile: talking.profile, name: talking.name }
}

/** The person's words, sent to whoever the view shows. */
export function said(talking: Talking, text: string): RemoteIn {
  return talking.kind === 'chat'
    ? { t: 'chatSay', project: talking.project, conversation: talking.conversation, text }
    : { t: 'sessionSay', profile: talking.profile, name: talking.name, text }
}

/** The messages shown, after something the machine said. */
export function after(messages: readonly Message[], talking: Talking, heard: RemoteOut): readonly Message[] {
  switch (heard.t) {
    case 'chat':
      if (talking.kind !== 'chat' || heard.conversation.id !== talking.conversation) return messages
      // A turn on its way is drawn from its frames; what was read meanwhile has none of it.
      return messages.some((one) => one.streaming) ? messages : heard.conversation.messages
    case 'chatFrame':
      if (talking.kind !== 'chat' || heard.conversation !== talking.conversation) return messages
      return applied(messages, heard.frame)
    case 'session':
      if (talking.kind !== 'session' || heard.profile !== talking.profile || heard.name !== talking.name) return messages
      return heard.messages
    default:
      return messages
  }
}

/** A piece's raw size: in base64 and its message it stays far under the
    256 KiB the machine takes from a device in one message. */
export const PIECE = 96 * 1024
/** What the machine takes, each: a photo as a paste, a recording of about two minutes. */
export const PHOTO_MOST = KEPT_BYTES
export const VOICE_MOST = 3 * 1024 * 1024
export const VOICE_SECONDS = 120

/** Why a photo is not sent, or nothing. */
export function photoRefused(photo: Blob): string | null {
  if (keptAsPicture(photo)) return null
  if (photo.size > PHOTO_MOST && photo.type.startsWith('image/')) return `that photo is over ${PHOTO_MOST / 1_048_576} MB`
  return 'a photo is a PNG, JPEG, GIF or WebP'
}

/** Why a recording is not sent, or nothing. */
export function voiceRefused(voice: Blob): string | null {
  if (!voice.type.startsWith('audio/')) return 'that is not a recording'
  if (voice.size > VOICE_MOST) return `a recording is at most ${VOICE_MOST / 1_048_576} MB, about two minutes`
  return null
}

/** A file in the pieces it is sent in, then the message that finishes it. */
export function pieces(bytes: Uint8Array, upload: string, finish: RemoteIn): RemoteIn[] {
  const parts: RemoteIn[] = []
  for (let at = 0; at < bytes.length; at += PIECE) parts.push({ t: 'uploadPart', upload, b64: base64(bytes.subarray(at, at + PIECE)) })
  return [...parts, finish]
}

function base64(bytes: Uint8Array): string {
  let binary = ''
  for (let at = 0; at < bytes.length; at += 0x8000) binary += String.fromCharCode(...bytes.subarray(at, at + 0x8000))
  return btoa(binary)
}

export const uploadId = (): string => `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`

/** The longest side a photo is sent at: more than a model reads, and a
    phone's camera makes several times it, too big for the paste's ceiling. */
const PHOTO_SIDE = 2048

/** A photo made small enough to send, as a JPEG; the photo as it is when
    it is small already or this browser cannot draw it. */
export async function shrunk(photo: Blob): Promise<Blob> {
  if (keptAsPicture(photo) && photo.size <= 1_048_576) return photo
  try {
    const drawn = await createImageBitmap(photo)
    const scale = Math.min(1, PHOTO_SIDE / Math.max(drawn.width, drawn.height))
    const canvas = document.createElement('canvas')
    canvas.width = Math.round(drawn.width * scale)
    canvas.height = Math.round(drawn.height * scale)
    canvas.getContext('2d')?.drawImage(drawn, 0, 0, canvas.width, canvas.height)
    drawn.close()
    const made = await new Promise<Blob | null>((done) => canvas.toBlob(done, 'image/jpeg', 0.85))
    return made ?? photo
  } catch {
    return photo
  }
}
