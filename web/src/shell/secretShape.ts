/*
 * Text that looks like a key, said before it is sent: a chat keeps what it is
 * sent in its transcript. The shapes are the vault's (apps/desktop/src/secret_scan.rs).
 */

const PREFIXED: ReadonlyArray<readonly [string, number]> = [
  ['sk-', 20],
  ['sk_live_', 20],
  ['rk_live_', 20],
  ['ghp_', 30],
  ['gho_', 30],
  ['github_pat_', 30],
  ['hf_', 20],
  ['xoxb-', 20],
  ['xoxp-', 20],
  ['glpat-', 20],
]

function secret(word: string): boolean {
  const value = word.replace(/^["'`(]+|["'`,;)]+$/g, '').split('=').pop() ?? ''
  if (PREFIXED.some(([prefix, shortest]) => value.startsWith(prefix) && value.length >= shortest)) return true
  const aws = value.length === 20 && /^(AKIA|ASIA)[A-Z0-9]{16}$/.test(value)
  const jwt = value.startsWith('eyJ') && value.split('.').length === 3 && value.length > 40
  return aws || jwt || /-----BEGIN [A-Z ]*PRIVATE KEY-----/.test(word)
}

/** Whether `text` holds something shaped like a key. */
export const looksSecret = (text: string): boolean =>
  /-----BEGIN [A-Z ]*PRIVATE KEY-----/.test(text) || text.split(/\s+/).some(secret)
