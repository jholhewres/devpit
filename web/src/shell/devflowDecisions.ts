import type { DevflowDecisions } from '../gen/bindings'

/*
 * Where Decisions stands for a project's Devflow, said the same on the Jev
 * panel and on the card as in the replies the agent reads.
 */

export function decisionsLine(decisions: DevflowDecisions): string {
  const why = decisions.reason ?? ''
  switch (decisions.state) {
    case 'on':
      return 'on — Jev answers'
    case 'off':
      return why ? `off — ${why}` : 'off'
    case 'no_key':
      return why || 'no key — set one in Settings → Decisions'
    case 'failing':
      return `failing: ${why || 'no reason given'}`
  }
}

/** The card's colour for it: a key missing warns, a failing call is red. */
export function decisionsTone(decisions: DevflowDecisions): 'ok' | 'lost' | 'failed' {
  if (decisions.state === 'failing') return 'failed'
  return decisions.state === 'no_key' ? 'lost' : 'ok'
}
