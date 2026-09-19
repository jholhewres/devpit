/*
 * What a person typed into an address bar, and where it points.
 *
 * Apart from the pane because it is the part that is easy to get subtly wrong
 * and impossible to notice from reading: `localhost:3000` is a url whose
 * scheme is `localhost`, and handing that to a parser gives a scheme nobody
 * asked for rather than the page somebody wanted.
 *
 * **devpit does not search.** A word that is not an address stays refused
 * rather than being sent to somebody's search engine — the pane opens what the
 * project is running, and quietly shipping a query to a third party is not
 * something an address bar should decide on its own.
 */

/** What an address bar was pointed at, or why it was not. */
export type Aimed = { at: string } | { refused: string }

/* The schemes the Rust side will accept. Repeated here on purpose: the window
   should say no before a round trip, and `browser.rs` says no again because a
   check only the window makes is a check a caller can skip. */
const OPENS = ['http:', 'https:']

/** Whether a scheme is one the pane can show. */
const opens = (scheme: string): boolean => OPENS.includes(scheme)

/*
 * A bare `host`, `host:port` or `host/path`, which is what people type.
 *
 * Deliberately narrow: letters, digits, dots and dashes before an optional
 * port and an optional path. `localhost`, `127.0.0.1:3000` and `example.com`
 * pass; a sentence with a space does not, and neither does something with a
 * scheme in it, which took the branch above.
 *
 * An IPv6 literal in brackets is allowed too — `[::1]:8080`. Added because
 * `loopback()` below was already testing for one that this could never
 * produce, which is dead code wearing the costume of coverage.
 */
const HOSTISH =
  /^([a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*|\[[0-9a-fA-F:]+\])(:\d{1,5})?(\/\S*)?$/

/**
 * Where to point the pane, given what somebody typed.
 *
 * Three cases, in this order: something that already carries a scheme, which
 * is honoured or refused on that scheme; something host-shaped, which gets
 * `https://` because that is the safe half of the guess; and anything else,
 * which is refused by name.
 */
export function aim(typed: string): Aimed {
  const text = typed.trim()
  if (!text) return { refused: 'Type an address to open.' }

  /* A scheme is `word:`, and only when what follows is not just digits —
     otherwise `localhost:3000` reads as the scheme `localhost`. */
  const scheme = /^([a-zA-Z][a-zA-Z0-9+.-]*):(?!\d+(?:\/|$))/.exec(text)
  if (scheme) {
    const named = `${scheme[1].toLowerCase()}:`
    if (!opens(named)) {
      return { refused: `This pane opens http and https. ${scheme[1]} is not one of them.` }
    }
    try {
      return { at: new URL(text).toString() }
    } catch {
      return { refused: `${text} is not an address.` }
    }
  }

  if (HOSTISH.test(text)) {
    /* `http` for a machine that is this one, `https` for everything else.
       The pane's own placeholder is `localhost:3000`, and typing it opened
       `https://localhost:3000/`, which essentially no local dev server
       answers — the pane failed at the one thing US-025 says it is for. A
       loopback address is not a place a downgrade attack can reach. */
    const scheme = loopback(text) ? 'http' : 'https'
    try {
      return { at: new URL(`${scheme}://${text}`).toString() }
    } catch {
      return { refused: `${text} is not an address.` }
    }
  }

  return { refused: `${text} is not an address. devpit does not search for you.` }
}

/** Whether a host is this machine, which is the only place `http` is assumed. */
const loopback = (text: string): boolean => {
  const authority = text.split('/')[0].toLowerCase()
  /* Brackets first: an IPv6 literal is full of the colon that splits a port. */
  if (authority.startsWith('[')) {
    const inside = authority.slice(1, authority.indexOf(']'))
    return inside === '::1' || inside === '0:0:0:0:0:0:0:1'
  }
  const host = authority.split(':')[0]
  return host === 'localhost' || host === '127.0.0.1' || host.endsWith('.localhost')
}

/** What to show in the bar for a url, which is the url and not a decoration. */
export function shown(url: string): string {
  try {
    const parsed = new URL(url)
    /* The trailing slash a parser adds to a bare host is noise nobody typed. */
    return parsed.pathname === '/' && !parsed.search && !parsed.hash
      ? `${parsed.protocol}//${parsed.host}`
      : parsed.toString()
  } catch {
    return url
  }
}

/** Where a pane sits in the window, which is what the native webview needs. */
export type Where = { x: number; y: number; width: number; height: number }

/**
 * The box a pane occupies, in window coordinates.
 *
 * A native webview is placed against the window and knows nothing about the
 * layout that decided where it goes, so the number has to be measured rather
 * than derived. A pane that has not been laid out yet measures zero, and the
 * Rust side clamps that to something a window can hold — a webview of zero
 * height is one nobody can see and nobody can close.
 */
export function boxOf(rect: DOMRect): Where {
  return {
    x: Math.round(rect.left),
    y: Math.round(rect.top),
    width: Math.round(rect.width),
    height: Math.round(rect.height),
  }
}

/** Whether two boxes differ enough to be worth telling the window about. */
export function moved(before: Where | null, now: Where): boolean {
  if (!before) return true
  return (
    before.x !== now.x ||
    before.y !== now.y ||
    before.width !== now.width ||
    before.height !== now.height
  )
}

/**
 * The host of a url, for offering a domain somebody probably means.
 *
 * Offered and not assumed: it fills the field, and importing still needs the
 * field to be agreed to. A url that will not parse offers nothing rather than
 * offering a guess.
 */
export function hostOf(url: string): string {
  try {
    return new URL(url).hostname
  } catch {
    return ''
  }
}
