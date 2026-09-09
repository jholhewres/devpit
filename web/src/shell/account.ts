/* Who the window says you are.
 *
 * The name comes from settings.account, which is null until there is a server
 * to answer for it. Until then the row says "Signed in" rather than a name
 * nobody chose — a fabricated identity is worse than an honest blank. */
export interface Who {
  readonly name: string
  readonly email: string | null
  readonly initials: string
}

export function who(account: string | null): Who {
  if (!account) return { name: 'Signed in', email: null, initials: '·' }
  const name = account.split('@')[0] ?? account
  const initials = name
    .split(/[.\-_\s]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]!.toUpperCase())
    .join('')
  return { name, email: account, initials: initials || name[0]!.toUpperCase() }
}
