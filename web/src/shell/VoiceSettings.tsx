import { open as pickFile } from '@tauri-apps/plugin-dialog'
import { useEffect, useState } from 'react'

import type { Transcribing } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * How voice messages become words: a whisper on this machine, a service the
 * person chose with their own key, or not at all. Nothing is sent anywhere
 * unless the service is chosen here.
 */

const ENGINES: readonly { id: string; label: string }[] = [
  { id: 'local', label: 'This machine' },
  { id: 'api', label: 'A service' },
  { id: 'off', label: 'Off' },
]

export function VoiceSettings(): React.JSX.Element {
  const [now, setNow] = useState<Transcribing | null>(null)
  const [key, setKey] = useState('')
  const [said, setSaid] = useState<string | null>(null)

  useEffect(() => {
    void ask(() => commands.transcribeRead()).then((answer) => setNow(answer.data))
  }, [])

  const save = (next: Transcribing): void => {
    setNow(next)
    void ask(() => commands.transcribeSet(next.engine, next.model, next.language, next.url)).then((answer) => {
      setSaid(answer.error)
      if (answer.data) setNow(answer.data)
    })
  }

  const keep = (value: string | null): void => {
    void ask(() => commands.transcribeKeySet(value)).then((answer) => {
      setSaid(answer.error)
      if (answer.error || !now) return
      setKey('')
      setNow({ ...now, keySet: value !== null })
    })
  }

  if (!now) return <></>
  const local = now.engine === 'local'
  const api = now.engine === 'api'
  const needsFile = now.local === 'whisper-cli' || now.local === 'whisper-cpp'
  return (
    <div className="pref pref--stack">
      <span className="pref__body">
        <span className="pref__t">Voice messages</span>
        <span className="pref__d">
          The microphone sits beside the paperclip in a chat&rsquo;s composer (Ctrl+Shift+A attaches files); a terminal has neither. Click or hold it to speak, and the words land in the composer to read before sending. Without an engine the recording goes as a file.
        </span>
        <span className="voice__row" role="radiogroup" aria-label="Engine">
          {ENGINES.map((one) => (
            <button key={one.id} className="btn" data-on={now.engine === one.id ? 'true' : undefined} role="radio" aria-checked={now.engine === one.id} onClick={() => save({ ...now, engine: one.id })}>
              {one.label}
            </button>
          ))}
        </span>
        {local && (
          <span className="pref__d">
            {now.local ? `Found ${now.local}.` : 'No whisper found, then reopen Settings once one is installed:'}
          </span>
        )}
        {local && !now.local && (
          <ul className="prefhow">
            <li><b>whisper.cpp</b>: <code>whisper-cli</code> on the PATH, <code>ffmpeg</code> beside it, and a model file such as <code>ggml-base.bin</code> from the whisper.cpp releases, chosen here.</li>
            <li><b>whisper</b>: <code>pip install openai-whisper</code> (or <code>whisper-ctranslate2</code>), which fetches its <code>base</code> model the first time.</li>
          </ul>
        )}
        {local && needsFile && (
          <span className="voice__row">
            <span className="voice__val" title={now.model}>{now.model || 'No model file chosen'}</span>
            <button
              className="btn"
              onClick={() =>
                void pickFile({ multiple: false, filters: [{ name: 'whisper.cpp model', extensions: ['bin'] }] }).then(
                  (picked) => typeof picked === 'string' && save({ ...now, model: picked }),
                )
              }
            >
              Choose model
            </button>
          </span>
        )}
        {api && (
          <>
            <label className="voice__row">
              <span className="voice__l">Address</span>
              <input className="voice__in" placeholder="https://api.openai.com/v1" defaultValue={now.url} onBlur={(event) => event.target.value !== now.url && save({ ...now, url: event.target.value })} />
            </label>
            <label className="voice__row">
              <span className="voice__l">Model</span>
              <input className="voice__in" placeholder="gpt-4o-mini-transcribe" defaultValue={now.model} onBlur={(event) => event.target.value !== now.model && save({ ...now, model: event.target.value })} />
            </label>
            <span className="voice__row">
              <span className="voice__l">Key</span>
              <input className="voice__in" type="password" autoComplete="off" placeholder={now.keySet ? 'Kept — type to replace' : 'Paste the service’s key'} value={key} onChange={(event) => setKey(event.target.value)} />
              <button className="btn" disabled={!key.trim()} onClick={() => keep(key)}>
                Keep
              </button>
              {now.keySet && (
                <button className="btn" onClick={() => keep(null)}>
                  Forget
                </button>
              )}
            </span>
          </>
        )}
        {now.engine !== 'off' && (
          <label className="voice__row">
            <span className="voice__l">Language</span>
            <input className="voice__in voice__in--short" placeholder="system" defaultValue={now.language} onBlur={(event) => event.target.value !== now.language && save({ ...now, language: event.target.value })} />
          </label>
        )}
        {said && <span className="pref__d voice__bad">{said}</span>}
      </span>
    </div>
  )
}
