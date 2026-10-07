import { describe, expect, it } from 'vitest'

import type { LiveSession } from '../gen/bindings'
import { draftToSend } from './draftShortcut'

const one = (name: string, draft: string | null, waiting = false): LiveSession =>
  ({ name, draft, waiting: waiting ? { question: 'Pick?', options: [], cursor: 0 } : null }) as LiveSession

describe('"send it" in the orchestrator\'s chat', () => {
  it('sends the one draft waiting', () => {
    expect(draftToSend('envie', [one('api', 'go on'), one('web', null)])?.name).toBe('api')
    expect(draftToSend('Send it!', [one('api', 'go on')])?.name).toBe('api')
  })

  it('is a message for the orchestrator when it says more, or the draft is not one', () => {
    expect(draftToSend('send the PR link to api', [one('api', 'go on')])).toBeNull()
    expect(draftToSend('envie', [one('api', 'go on'), one('web', 'stop')])).toBeNull()
    expect(draftToSend('envie', [one('api', 'go on', true)])).toBeNull()
    expect(draftToSend('envie', [])).toBeNull()
  })
})
