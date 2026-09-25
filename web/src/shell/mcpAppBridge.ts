/*
 * The host's half of MCP Apps (SEP-1865): what a page asks of devpit over
 * `postMessage`, as JSON-RPC 2.0, and what it is answered.
 *
 * Kept apart from the iframe so it is tested as data: a message in, an
 * answer out. Only what devpit does is answered; everything else is "method
 * not found", which a page is written to expect from a host that does less.
 */

export interface Rpc {
  readonly jsonrpc?: string
  readonly id?: number | string
  readonly method?: string
  readonly params?: Record<string, unknown>
}

export interface Bridge {
  /** The tool call the page belongs to. */
  readonly input: unknown
  readonly theme: 'light' | 'dark'
  /** Runs a tool of the page's own server, once the person has let it. */
  call: (name: string, args: unknown) => Promise<unknown>
  openLink: (url: string) => void
  resize: (height: number) => void
  /** The page is ready for the call's input and, once there, its result. */
  ready: () => void
}

export const PROTOCOL = '2026-01-26'

const reply = (id: Rpc['id'], result: unknown): Rpc & { result: unknown } => ({ jsonrpc: '2.0', id, result })
const refuse = (id: Rpc['id'], code: number, message: string): Rpc & { error: { code: number; message: string } } => ({
  jsonrpc: '2.0',
  id,
  error: { code, message },
})

/** The answer to one message from the page, or null for a notification. */
export async function answer(message: Rpc, bridge: Bridge): Promise<object | null> {
  const { id, method, params = {} } = message
  const notification = id === undefined

  switch (method) {
    case 'ui/initialize':
      return reply(id, {
        protocolVersion: typeof params.protocolVersion === 'string' ? params.protocolVersion : PROTOCOL,
        hostInfo: { name: 'devpit', version: '1' },
        hostCapabilities: { openLinks: {}, serverTools: {} },
        hostContext: { theme: bridge.theme, displayMode: 'inline', availableDisplayModes: ['inline'], platform: 'desktop' },
      })
    case 'ui/notifications/initialized':
      bridge.ready()
      return null
    case 'ui/notifications/size-changed': {
      const height = Number(params.height)
      if (Number.isFinite(height)) bridge.resize(height)
      return null
    }
    case 'ping':
      return reply(id, {})
    case 'ui/open-link': {
      const url = String(params.url ?? '')
      if (!/^https?:\/\//i.test(url)) return refuse(id, -32602, 'only web links open')
      bridge.openLink(url)
      return reply(id, {})
    }
    case 'tools/call': {
      const name = String(params.name ?? '')
      if (!name) return refuse(id, -32602, 'which tool?')
      try {
        return reply(id, await bridge.call(name, params.arguments ?? {}))
      } catch (error) {
        return refuse(id, -32000, error instanceof Error ? error.message : String(error))
      }
    }
    default:
      return notification ? null : refuse(id, -32601, `devpit does not do ${method ?? 'that'}`)
  }
}

/** The notifications that hand the page its call: the input, then — when
 *  there is one — the result, in the shape `tools/call` answers. */
export function toolInput(input: unknown): Rpc {
  return { jsonrpc: '2.0', method: 'ui/notifications/tool-input', params: { arguments: input } }
}

export function toolResult(output: string, isError: boolean): Rpc {
  let structured: unknown
  try {
    structured = JSON.parse(output)
  } catch {
    structured = undefined
  }
  return {
    jsonrpc: '2.0',
    method: 'ui/notifications/tool-result',
    params: {
      content: [{ type: 'text', text: output }],
      ...(structured && typeof structured === 'object' ? { structuredContent: structured } : {}),
      isError,
    },
  }
}
