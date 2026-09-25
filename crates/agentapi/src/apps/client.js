// devpit's half of an MCP App page: talks to the host over postMessage,
// JSON-RPC 2.0, as the MCP Apps extension says. Every text is set with
// textContent; nothing a card or a session says becomes markup.
const devpit = (() => {
  let next = 1
  const waiting = new Map()
  const handlers = { input: () => {}, result: () => {} }
  const send = (message) => window.parent.postMessage({ jsonrpc: '2.0', ...message }, '*')
  const request = (method, params) =>
    new Promise((resolve, reject) => {
      const id = next++
      waiting.set(id, { resolve, reject })
      send({ id, method, params })
    })
  window.addEventListener('message', (event) => {
    if (event.source !== window.parent) return
    const m = event.data || {}
    if (m.id !== undefined && waiting.has(m.id)) {
      const { resolve, reject } = waiting.get(m.id)
      waiting.delete(m.id)
      return m.error ? reject(new Error(m.error.message || 'refused')) : resolve(m.result)
    }
    if (m.method === 'ui/notifications/tool-input') handlers.input(m.params && m.params.arguments)
    if (m.method === 'ui/notifications/tool-result') handlers.result(m.params || {})
  })
  const size = () => send({ method: 'ui/notifications/size-changed', params: { height: document.documentElement.scrollHeight } })
  new ResizeObserver(size).observe(document.body)
  const parse = (result) => {
    if (result && result.structuredContent) return result.structuredContent
    const text = result && result.content && result.content[0] && result.content[0].text
    try { return JSON.parse(text) } catch { return text }
  }
  return {
    start: async (on) => {
      Object.assign(handlers, on)
      const hello = await request('ui/initialize', { protocolVersion: '2026-01-26', appInfo: { name: 'devpit', version: '1' }, appCapabilities: {} })
      const theme = hello && hello.hostContext && hello.hostContext.theme
      if (theme) document.documentElement.dataset.theme = theme
      send({ method: 'ui/notifications/initialized', params: {} })
    },
    // A devpit tool, through the host; its answer as data, or its error.
    call: async (name, args) => {
      const result = await request('tools/call', { name, arguments: args || {} })
      if (result && result.isError) throw new Error(parse(result))
      return parse(result)
    },
    parse,
    open: (url) => request('ui/open-link', { url }),
    el: (tag, props, ...kids) => {
      const node = document.createElement(tag)
      for (const [key, value] of Object.entries(props || {})) {
        if (key === 'text') node.textContent = value
        else if (key.startsWith('on')) node.addEventListener(key.slice(2), value)
        else if (value !== undefined && value !== null && value !== false) node.setAttribute(key, value === true ? '' : value)
      }
      for (const kid of kids.flat()) if (kid) node.append(kid)
      return node
    },
    show: (...nodes) => document.getElementById('app').replaceChildren(...nodes.flat().filter(Boolean)),
    size,
  }
})()
