import type { Doing, SessionKind } from '../gen/bindings'

/*
 * What Open as chat may do with one of a card's sessions.
 *
 * A session still running is not taken: two CLIs going on with one session
 * would each write their own continuation of it. A background session is
 * taken only once it has finished or gone — `open` is a live process waiting
 * for input, which is still running. A pane goes on from its own terminal,
 * where continuing closes the pane first.
 */

export type Adoption = 'adopt' | 'stop-first' | 'in-terminal' | 'chat'

export function adoptable(kind: SessionKind, state: Doing | null): Adoption {
  switch (kind) {
    case 'pane':
      return 'in-terminal'
    case 'chat':
      return 'chat'
    case 'run':
      return state === 'working' ? 'stop-first' : 'adopt'
    case 'background':
      return state === 'done' || state === 'gone' ? 'adopt' : 'stop-first'
  }
}
