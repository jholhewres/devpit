import { useShell } from './useShell'

/*
 * The first step of the first run: the account, and the wait for it.
 *
 * Its own file because the wait has three faces and `Onboarding.tsx` has two
 * more steps after this one — the offer, the code to approve, and the refusal
 * to say what went wrong. Kept together they put that file past its ceiling,
 * which is the ceiling doing its job.
 *
 * The approval happens in a browser and lands back here, so nothing on this
 * screen advances on the click that opened it. It used to: the code was never
 * shown, and whoever approved it came back to a theme picker with no way to
 * know whether it had worked.
 */
export function OnboardingAccount({ onSkip }: { onSkip: () => void }): React.JSX.Element {
  const { signIn, membership } = useShell()
  const { signingIn, failed } = membership

  if (signingIn) {
    return (
      <>
        <h1 className="onb__t">Approve this code</h1>
        <p className="onb__d">
          Your browser is open at <b>{host(membership.origin)}</b>. Check that it shows this code,
          then approve it there — this screen moves on by itself.
        </p>
        <div className="onb__code">{signingIn.userCode}</div>
        <p className="onb__wait" role="status">
          <span className="onb__pulse" aria-hidden="true" />
          Waiting for you to approve&hellip;
        </p>
        <button
          className="onb__skip"
          onClick={() => {
            membership.cancelSignIn()
            onSkip()
          }}
        >
          Not now
        </button>
      </>
    )
  }

  return (
    <>
      <h1 className="onb__t">Sign in, if you want to</h1>
      <p className="onb__d">
        Free, and optional. Today it signs you in and nothing more — syncing between machines is
        not built yet. <b>Your projects, conversations and files stay on this computer</b> either
        way.
      </p>
      {/* A refusal used to leave with the screen. It stays now, and the button
          says what it would be doing this time. */}
      {failed && (
        <p className="onb__failed" role="alert">
          {failed}
        </p>
      )}
      <button className="onb__go" onClick={() => signIn()}>
        {failed ? 'Try again' : 'Sign in'}
      </button>
      <button className="onb__skip" onClick={onSkip}>
        Not now
      </button>
    </>
  )
}

/* The host on its own, as in `SignIn.tsx`: somebody checking that the page in
   front of them is the one the app opened does not need the scheme to do it. */
function host(origin: string): string {
  try {
    return new URL(origin).host
  } catch {
    return origin
  }
}
