/*
 * Skills picked for a turn, and the prompt they turn into.
 *
 * Claude Code lists skills among its slash commands (the recorded init line
 * carries them in `slash_commands`), and a prompt runs one command — the one
 * on its first line. So one skill becomes that line, and several become a
 * sentence naming them, which the agent reads and a slash parser cannot trip on.
 */

export function withSkills(prompt: string, skills: readonly string[]): string {
  const unique = [...new Set(skills)]
  if (unique.length === 0) return prompt
  if (unique.length === 1) return `/${unique[0]}\n${prompt}`
  const named = `${unique.slice(0, -1).join(', ')} and ${unique.at(-1)}`
  return `Use the ${named} skills.\n${prompt}`
}

/* A skill once, however many times it was picked. */
export function added(skills: readonly string[], skill: string): readonly string[] {
  return skills.includes(skill) ? skills : [...skills, skill]
}

export const removed = (skills: readonly string[], skill: string): readonly string[] =>
  skills.filter((one) => one !== skill)
