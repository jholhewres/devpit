import { describe, expect, it, vi } from 'vitest'

import { browserInternals, Events, rpc } from './transport'

const answered = (status: number, body: string): typeof fetch =>
  vi.fn(async () => new Response(body, { status })) as unknown as typeof fetch

describe('a command in a browser', () => {
  it('is a POST to /rpc with its arguments, answered with the command body', async () => {
    const send = answered(200, '{"projects":[]}')
    await expect(rpc('project_list', { front: null }, send)).resolves.toEqual({ projects: [] })
    expect(send).toHaveBeenCalledWith('/rpc/project_list', expect.objectContaining({
      method: 'POST',
      credentials: 'same-origin',
      body: '{"front":null}',
    }))
  })

  it('rejects with the command own error, as the window does', async () => {
    await expect(rpc('project_open', {}, answered(500, '{"message":"gone"}'))).rejects.toEqual({ message: 'gone' })
    await expect(rpc('chat_send', {}, answered(501, 'only in the desktop app'))).rejects.toEqual({
      message: 'only in the desktop app',
      status: 501,
    })
  })

  it('never carries an invoke key, only the cookie the browser holds', async () => {
    const send = answered(200, 'null')
    await browserInternals(new Events(() => ({}) as WebSocket), send).invoke('app_info', {})
    const [, init] = (send as unknown as { mock: { calls: [string, RequestInit][] } }).mock.calls[0]!
    expect(JSON.stringify(init)).not.toMatch(/invoke.?key/i)
  })

  it('refuses a plugin command, which is the desktop own UI', async () => {
    const send = answered(200, 'null')
    await expect(browserInternals(new Events(() => ({}) as WebSocket), send).invoke('plugin:dialog|open', {})).rejects.toMatchObject({
      status: 501,
    })
    expect(send).not.toHaveBeenCalled()
  })
})

class FakeSocket {
  readyState = 1
  sent: string[] = []
  onmessage: ((message: { data: string }) => void) | null = null
  onopen: (() => void) | null = null
  onclose: (() => void) | null = null
  send(text: string): void {
    this.sent.push(text)
  }
}

describe('an event in a browser', () => {
  it('is asked for by name and handed to each listener the way Tauri hands it', () => {
    vi.stubGlobal('WebSocket', { OPEN: 1 })
    const socket = new FakeSocket()
    const events = new Events(() => socket as unknown as WebSocket)
    const internals = browserInternals(events, answered(200, 'null'))
    const heard: unknown[] = []
    const handler = internals.transformCallback((event) => heard.push(event))
    void internals.invoke('plugin:event|listen', { event: 'board:changed', target: { kind: 'Any' }, handler })
    socket.onopen?.()
    expect(JSON.parse(socket.sent.at(-1)!)).toEqual({ listen: ['board:changed'] })

    socket.onmessage?.({ data: '{"t":"board:changed","p":"prj_1"}' })
    socket.onmessage?.({ data: '{"t":"run:changed","p":1}' })
    socket.onmessage?.({ data: 'not json' })
    expect(heard).toEqual([{ event: 'board:changed', id: 1, payload: 'prj_1' }])
    vi.unstubAllGlobals()
  })

  it('stops being asked for when its last listener leaves', async () => {
    vi.stubGlobal('WebSocket', { OPEN: 1 })
    const socket = new FakeSocket()
    const events = new Events(() => socket as unknown as WebSocket)
    const internals = browserInternals(events, answered(200, 'null'))
    const handler = internals.transformCallback(() => undefined)
    const id = (await internals.invoke('plugin:event|listen', { event: 'run:changed', handler })) as number
    socket.onopen?.()
    await internals.invoke('plugin:event|unlisten', { event: 'run:changed', eventId: id })
    expect(JSON.parse(socket.sent.at(-1)!)).toEqual({ listen: [] })
    vi.unstubAllGlobals()
  })
})
