/*
 * What the policy blocked, said out loud.
 *
 * A Content-Security-Policy fails silently by design: the request simply does
 * not happen, and a panel goes blank with nothing in the log to say why. The
 * browser does fire an event first, so this turns each one into a line naming
 * the directive that refused and what it refused — which is what the e2e suite
 * asserts is empty, and what a person debugging a blank canvas needs.
 *
 * Only the window's own fetches pass through the policy. `plan_limits` and the
 * account talk to the network from the Rust side through reqwest, so nothing
 * they do reaches `connect-src`.
 */

export function watchCsp(target: EventTarget = document): () => void {
  const heard = (event: Event): void => {
    const blocked = event as SecurityPolicyViolationEvent
    const what = blocked.blockedURI || 'something inline'
    console.error(`[csp] ${blocked.violatedDirective} blocked ${what}`)
  }
  target.addEventListener('securitypolicyviolation', heard)
  return () => target.removeEventListener('securitypolicyviolation', heard)
}
