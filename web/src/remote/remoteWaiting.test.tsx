import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { RemoteDraft, RemoteIn, RemoteStop } from '../gen/bindings'
import { RemoteWaiting } from './RemoteWaiting'

afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

const stop = (profile: string, name: string, losing: string | null = null): RemoteStop => ({ profile, stop: { name, pid: null, losing } })
const draft = (name: string, text: string): RemoteDraft => ({ profile: 'claude', name, text })

function shown(stops: RemoteStop[], drafts: RemoteDraft[], typing = true) {
  const send = vi.fn<(message: RemoteIn) => void>()
  vi.stubGlobal('confirm', vi.fn(() => true))
  render(<RemoteWaiting questions={[]} drafts={drafts} stops={stops} health={null} answering={false} typing={typing} send={send} />)
  return send
}

describe('what waits on the person, on the Remote', () => {
  it('lists the stops asked and stops them all on one press, as shown', () => {
    const stops = [stop('claude', 'api'), stop('claude', 'web', 'it is working right now'), stop('work', 'docs')]
    const send = shown(stops, [])
    expect(screen.getByText('api')).toBeTruthy()
    expect(screen.getByText('it is working right now')).toBeTruthy()
    expect(send).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Stop all 3' }))
    expect(send).toHaveBeenCalledTimes(1)
    expect(send).toHaveBeenCalledWith({ t: 'stopsConfirm', stops })
  })

  it('keeps them running without stopping any', () => {
    const stops = [stop('claude', 'api')]
    const send = shown(stops, [])
    fireEvent.click(screen.getByRole('button', { name: 'Keep running' }))
    expect(send).toHaveBeenCalledWith({ t: 'stopsDrop', stops })
    expect(send).not.toHaveBeenCalledWith(expect.objectContaining({ t: 'stopsConfirm' }))
  })

  it('sends every draft on one press', () => {
    const send = shown([], [draft('api', 'go on'), draft('web', 'run the tests')])
    fireEvent.click(screen.getByRole('button', { name: 'Send all 2' }))
    expect(send.mock.calls.map(([one]) => one)).toEqual([
      { t: 'draftSend', profile: 'claude', name: 'api' },
      { t: 'draftSend', profile: 'claude', name: 'web' },
    ])
  })

  it('offers no press to a device that may only watch', () => {
    shown([stop('claude', 'api')], [draft('api', 'go on'), draft('web', 'run the tests')], false)
    expect(screen.queryByRole('button', { name: /Stop|Send/ })).toBeNull()
  })
})
