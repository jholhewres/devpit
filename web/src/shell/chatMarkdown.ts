import type { Message, Part } from '../gen/bindings'

/*
 * A conversation as markdown: what was said, or what was said and every tool
 * it ran. Built from the messages, not from the screen, so a turn the window
 * has not drawn is exported like the rest.
 */

export type Exported = 'messages' | 'tools'

const WHO: Record<Message['role'], string> = { user: 'You', assistant: 'Agent', system: 'System' }

/* One backtick longer than any run inside, so nothing in the body can close it. */
export function fenced(body: string, language = ''): string {
  const longest = Math.max(2, ...[...body.matchAll(/`+/g)].map((run) => run[0].length))
  const fence = '`'.repeat(longest + 1)
  return `${fence}${language}\n${body}\n${fence}`
}

/* Prose that opened a fence and never closed it would swallow every message after it. */
export function closed(text: string): string {
  const open = text.split('\n').filter((line) => /^\s{0,3}```/.test(line)).length % 2 === 1
  return open ? `${text}\n\`\`\`` : text
}

const pretty = (input: string): string => {
  try {
    return JSON.stringify(JSON.parse(input), null, 2)
  } catch {
    return input
  }
}

function partOf(part: Part, tools: boolean): string | null {
  switch (part.kind) {
    case 'text':
      /* A subagent's words are not the conversation's. */
      return part.parent ? null : closed(part.text.trim())
    case 'tool_call':
      return tools ? `**${part.name}**\n\n${fenced(pretty(part.input), 'json')}` : null
    case 'tool_result':
      return tools ? `${part.is_error ? 'Failed' : 'Result'}:\n\n${fenced(part.output)}` : null
    case 'command':
      return tools ? fenced(part.content) : null
    default:
      return null
  }
}

export function markdownOf(title: string, messages: readonly Message[], what: Exported): string {
  const said = messages.flatMap((message) => {
    const parts = message.parts.map((part) => partOf(part, what === 'tools')).filter((one): one is string => Boolean(one))
    return parts.length ? [`## ${WHO[message.role]}\n\n${parts.join('\n\n')}`] : []
  })
  return `${[`# ${title.replace(/\n/g, ' ')}`, ...said].join('\n\n')}\n`
}
