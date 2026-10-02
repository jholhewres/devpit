/*
 * A voice message, recorded in the window: the microphone through
 * MediaRecorder, in the first format this webview can write.
 */

/** The longest a voice message runs before it stops on its own. */
export const LONGEST_MS = 5 * 60_000

const FORMATS = ['audio/webm;codecs=opus', 'audio/webm', 'audio/ogg;codecs=opus', 'audio/mp4']

/** The format to record in, or null where this webview records none. */
export function formatHere(): string | null {
  if (typeof MediaRecorder === 'undefined' || !navigator.mediaDevices?.getUserMedia) return null
  return FORMATS.find((one) => MediaRecorder.isTypeSupported(one)) ?? null
}

/** The language to hear it in when the settings name none: the system's, by its first part. */
export function systemLanguage(): string | null {
  const code = (navigator.language || '').split('-')[0]?.toLowerCase() ?? ''
  return /^[a-z]{2,3}$/.test(code) ? code : null
}

/** Why the microphone could not be had, in words. */
export function refusal(error: unknown): string {
  const name = error instanceof DOMException ? error.name : ''
  if (name === 'NotAllowedError' || name === 'SecurityError')
    return 'The microphone is blocked: allow devpit to use it in the system’s privacy settings, then try again.'
  if (name === 'NotFoundError') return 'No microphone was found.'
  return 'The microphone could not be opened.'
}

export interface Recording {
  /** Stops and answers with what was heard; null when nothing was. */
  stop: () => Promise<Blob | null>
  cancel: () => void
}

/** Starts recording; rejects when the microphone cannot be had. */
export async function record(format: string): Promise<Recording> {
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
  const recorder = new MediaRecorder(stream, { mimeType: format })
  const chunks: Blob[] = []
  recorder.ondataavailable = (event) => event.data.size > 0 && chunks.push(event.data)
  const done = new Promise<void>((resolve) => (recorder.onstop = () => resolve()))
  const release = (): void => stream.getTracks().forEach((track) => track.stop())
  recorder.start(1000)
  return {
    stop: async () => {
      if (recorder.state !== 'inactive') recorder.stop()
      await done
      release()
      return chunks.length > 0 ? new Blob(chunks, { type: format.split(';')[0] }) : null
    },
    cancel: () => {
      chunks.length = 0
      if (recorder.state !== 'inactive') recorder.stop()
      release()
    },
  }
}

/** A blob as the base64 the paste command takes. */
export function base64Of(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const url = typeof reader.result === 'string' ? reader.result : ''
      resolve(url.slice(url.indexOf(',') + 1))
    }
    reader.onerror = () => reject(reader.error)
    reader.readAsDataURL(blob)
  })
}
