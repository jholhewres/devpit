/* Who the window says you are.
 *
 * The account comes from the accounts server, which today knows an address and
 * a name it has usually not been given. Until it has one, the row is named
 * after the address — and with no account at all it says "Signed in" rather
 * than a name nobody chose, because a fabricated identity is worse than an
 * honest blank. */

import type { Account } from '../gen/bindings'

export interface Who {
  readonly name: string
  readonly email: string | null
  readonly initials: string
}

export function who(account: Account | null): Who {
  if (!account) return { name: 'Signed in', email: null, initials: '·' }

  // A name the person set wins over one derived from their address: they chose
  // one of the two.
  const name = account.name?.trim() || (account.email.split('@')[0] ?? account.email)
  return { name, email: account.email, initials: initialsOf(name) }
}

function initialsOf(name: string): string {
  const initials = name
    .split(/[.\-_\s]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]!.toUpperCase())
    .join('')
  return initials || name[0]!.toUpperCase()
}
