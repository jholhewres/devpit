/*
 * devpit.app/remote is only a launcher: it lists the machines a person added
 * and opens each one's own address. What it is told travels in the fragment,
 * which a browser never sends to the server — no token, no session data.
 */

export const LAUNCHER = 'https://devpit.app/remote'

/** The machine's name, out of its tailnet address: its first label. */
export function machineName(address: string): string {
  try {
    return new URL(address).hostname.split('.')[0] ?? address
  } catch {
    return address
  }
}

/** The link that adds this machine to the launcher, or null without an HTTPS address. */
export function launcherLink(address: string, name = machineName(address)): string | null {
  let url: URL
  try {
    url = new URL(address)
  } catch {
    return null
  }
  if (url.protocol !== 'https:') return null
  const said = new URLSearchParams({ add: url.origin, name })
  return `${LAUNCHER}#${said.toString()}`
}
