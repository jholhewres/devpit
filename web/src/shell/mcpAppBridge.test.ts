import { describe, expect, it, vi } from 'vitest'

import { answer, toolResult, type Bridge } from './mcpAppBridge'

const bridge = (over: Partial<Bridge> = {}): Bridge => ({
  input: { issueKey: 'LED-1' },
  theme: 'dark',
  call: vi.fn(async () => ({ content: [] })),
  openLink: vi.fn(),
  resize: vi.fn(),
  ready: vi.fn(),
  ...over,
})

describe('what a page is answered', () => {
  it('is told who the host is, in the version it spoke', async () => {
    const said = (await answer({ jsonrpc: '2.0', id: 1, method: 'ui/initialize', params: { protocolVersion: '2026-01-26' } }, bridge())) as { result: { protocolVersion: string; hostInfo: { name: string }; hostContext: { theme: string } } }
    expect(said.result.protocolVersion).toBe('2026-01-26')
    expect(said.result.hostInfo.name).toBe('devpit')
    expect(said.result.hostContext.theme).toBe('dark')
  })

  it('gets its call once it says it is ready, and grows to what it asks', async () => {
    const one = bridge()
    expect(await answer({ jsonrpc: '2.0', method: 'ui/notifications/initialized' }, one)).toBeNull()
    expect(one.ready).toHaveBeenCalled()
    await answer({ jsonrpc: '2.0', method: 'ui/notifications/size-changed', params: { height: 420 } }, one)
    expect(one.resize).toHaveBeenCalledWith(420)
  })

  it('opens only web links', async () => {
    const one = bridge()
    expect(await answer({ jsonrpc: '2.0', id: 2, method: 'ui/open-link', params: { url: 'javascript:alert(1)' } }, one)).toMatchObject({ error: { code: -32602 } })
    expect(one.openLink).not.toHaveBeenCalled()
    await answer({ jsonrpc: '2.0', id: 3, method: 'ui/open-link', params: { url: 'https://jira.example/LED-1' } }, one)
    expect(one.openLink).toHaveBeenCalledWith('https://jira.example/LED-1')
  })

  it("passes a tool call on, and says so when it is refused", async () => {
    const one = bridge({ call: vi.fn(async () => { throw new Error('not allowed') }) })
    expect(await answer({ jsonrpc: '2.0', id: 4, method: 'tools/call', params: { name: 'getJiraIssue', arguments: { key: 'LED-1' } } }, one)).toMatchObject({ error: { message: 'not allowed' } })
    expect(one.call).toHaveBeenCalledWith('getJiraIssue', { key: 'LED-1' })
  })

  it('answers what it does not do with method-not-found, and ignores unknown notifications', async () => {
    expect(await answer({ jsonrpc: '2.0', id: 5, method: 'ui/message' }, bridge())).toMatchObject({ error: { code: -32601 } })
    expect(await answer({ jsonrpc: '2.0', method: 'notifications/whatever' }, bridge())).toBeNull()
  })
})

describe('the result a page is handed', () => {
  it('carries the structured content when the tool answered JSON', () => {
    expect(toolResult('{"issue":{"key":"LED-1"}}', false).params).toMatchObject({ structuredContent: { issue: { key: 'LED-1' } }, isError: false })
    expect(toolResult('plain words', true).params).not.toHaveProperty('structuredContent')
  })
})
