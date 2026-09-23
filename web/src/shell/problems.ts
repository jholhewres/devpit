/*
 * Why something asked for from a menu did not happen.
 *
 * Reveal, Open in, a file created or renamed: each fired its command and
 * dropped the answer, so a refusal looked exactly like a click that did
 * nothing. An event, like `useTree.changed`, because the menus that act and
 * the one place that says so are nowhere near each other.
 */

export const PROBLEM = 'devpit:problem'

export const report = (why: string): void => {
  window.dispatchEvent(new CustomEvent<string>(PROBLEM, { detail: why }))
}

/** Says why when the answer is a refusal. Whether it went through. */
export function went(answer: { readonly error: string | null }): boolean {
  if (answer.error) report(answer.error)
  return answer.error === null
}
