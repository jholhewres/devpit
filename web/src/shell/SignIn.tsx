import { useEffect } from 'react'

import mark from '../assets/brand/mark.png'
import { useShell } from './useShell'

/* Signing in.
 *
 * One button, because there is one way in: the browser. devpit never sees a
 * password — it asks the accounts site for a short code, opens the browser on
 * the page that approves it, and waits. Whatever the site grows later (GitHub,
 * Google, a passkey) arrives without this sheet changing at all.
 */
export function SignIn({ onClose }: { onClose: () => void }): React.JSX.Element {
  const { membership } = useShell()
  const { signingIn, failed, origin, account } = membership

  /* The sheet's job ends when there is an account. Closing on the click that
     opened the browser would have closed it before anything happened. */
  useEffect(() => {
    if (account) onClose()
  }, [account, onClose])

  const close = (): void => {
    membership.cancelSignIn()
    onClose()
  }

  return (
    <div
      className="auth"
      data-open="true"
      onClick={(event) => event.target === event.currentTarget && close()}
    >
      <div className="auth__box" role="dialog" aria-label="Sign in to devpit">
        <button className="auth__x" aria-label="Close" onClick={close}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
        <span className="auth__mark"><img className="mark" alt="" src={mark} /></span>

        {signingIn ? (
          <>
            <h2 className="auth__t">Approve this code</h2>
            <p className="auth__d">
              Your browser is open at <b>{host(origin)}</b>. Check that it shows this code, then
              approve it there.
            </p>
            <div className="auth__code">{signingIn.userCode}</div>
            <p className="auth__fine">Waiting for you to approve&hellip;</p>
          </>
        ) : (
          <>
            <h2 className="auth__t">Sign in to devpit</h2>
            <p className="auth__d">
              Free. Your projects, conversations and files stay on this computer &mdash; an account
              saves the workspace around them.
            </p>
            {failed && (
              <p className="auth__failed" role="alert">
                {failed}
              </p>
            )}
            <button className="auth__b" onClick={() => void membership.signIn()}>
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4M10 17l5-5-5-5M15 12H3" /></svg>
              Continue in your browser
            </button>
            <p className="auth__fine">
              Signing in creates an account the first time. devpit never sees your password.
            </p>
          </>
        )}
      </div>
    </div>
  )
}

/* The host on its own: a person checking that the page in front of them is the
   one the app opened does not need the scheme to do it. */
function host(origin: string): string {
  try {
    return new URL(origin).host
  } catch {
    return origin
  }
}
