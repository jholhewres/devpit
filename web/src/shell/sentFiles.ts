/*
 * What a person sent, split back into what they attached and what they
 * typed. `withFiles` led the prompt with one `@path` per file, and a picked
 * skill (`withSkills`) puts its own line above that.
 */

export interface SentFile {
  readonly path: string
  readonly name: string
  /** The extension, lowercased. */
  readonly kind: string
  readonly image: boolean
}

const PICTURES = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp'])

const files = (line: string | undefined): SentFile[] | null => {
  if (!line?.startsWith('@')) return null
  const tokens = line.split(' ')
  if (!tokens.every((token) => token.length > 1 && token.startsWith('@'))) return null
  return tokens.map((token) => {
    const path = token.slice(1)
    const name = path.split(/[\\/]/).filter(Boolean).at(-1) ?? path
    const dot = name.lastIndexOf('.')
    const kind = dot > 0 ? name.slice(dot + 1).toLowerCase() : ''
    return { path, name, kind, image: PICTURES.has(kind) }
  })
}

export function sentWith(text: string): { files: readonly SentFile[]; prompt: string } {
  const lines = text.split('\n')
  const first = files(lines[0])
  if (first) return { files: first, prompt: lines.slice(1).join('\n') }
  const skilled = lines[0]!.startsWith('/') || lines[0]!.startsWith('Use the ')
  const second = skilled ? files(lines[1]) : null
  if (second) return { files: second, prompt: [lines[0], ...lines.slice(2)].join('\n') }
  return { files: [], prompt: text }
}

/* The line `withSkills` puts above the words: `/name`, or "Use the a and b skills." */
const SKILL_LINE = /^(\/[\w:.-]+|Use the .+ skills\.)$/

/* What copying a sent message gives: the words as typed, without the skill
   line above them, and the files it went with named after them. */
export function copiedOf(text: string): string {
  const { files, prompt } = sentWith(text)
  const lines = prompt.split('\n')
  const words = lines.length > 1 && SKILL_LINE.test(lines[0]!) ? lines.slice(1).join('\n') : prompt
  if (files.length === 0) return words
  return [words, `Attached:\n${files.map((file) => `- ${file.path}`).join('\n')}`].filter(Boolean).join('\n\n')
}

/* Past this many lines a message folds, with a fade over where it was cut. */
export const LONG_LINES = 14
export const isLong = (prompt: string): boolean => prompt.split('\n').length > LONG_LINES || prompt.length > 1400
