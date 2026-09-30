/*
 * A launch line as a list shows it: the program and its flags, without the
 * variables in front. Those are an account's own directory, an endpoint, a
 * token — a home path at best and a secret at worst, and neither says which
 * agent a row is.
 */

/* One `NAME=value` in front of the program, the value quoted or not. */
const ASSIGNMENT = /^\s*[A-Za-z_][A-Za-z0-9_]*=(?:'[^']*'|"[^"]*"|\S*)\s+/

export function launchShown(line: string): string {
  let rest = line
  while (ASSIGNMENT.test(rest)) rest = rest.replace(ASSIGNMENT, '')
  return rest.trim()
}
