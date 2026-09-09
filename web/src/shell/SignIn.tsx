import mark from '../assets/brand/mark.png'

/* The sheet stands in for the identity providers. Nothing is collected
   here, and nothing will be: the flow is OAuth in the browser. */

export function SignIn({ onClose, onSignIn }: { onClose: () => void; onSignIn: () => void }): React.JSX.Element {
  return (
    <div className="auth" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="auth__box" role="dialog" aria-label="Sign in to devpit">
        <button className="auth__x" aria-label="Close" onClick={onClose}><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
        <span className="auth__mark"><img className="mark" alt="" src={mark} /></span>
        <h2 className="auth__t">Sign in to devpit</h2>
        <p className="auth__d">Free. Your projects, conversations and files stay on this
          computer &mdash; an account saves the workspace around them.</p>
        <button className="auth__b" onClick={onSignIn}><svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.45-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.9 1.53 2.36 1.09 2.94.83.09-.65.35-1.09.63-1.34-2.22-.25-4.56-1.11-4.56-4.95 0-1.09.39-1.99 1.03-2.69-.1-.25-.45-1.27.1-2.65 0 0 .84-.27 2.75 1.03a9.5 9.5 0 0 1 5 0c1.91-1.3 2.75-1.03 2.75-1.03.55 1.38.2 2.4.1 2.65.64.7 1.03 1.6 1.03 2.69 0 3.85-2.34 4.7-4.57 4.95.36.31.68.92.68 1.85v2.74c0 .26.18.58.69.48A10 10 0 0 0 12 2Z" /></svg>Continue with GitHub</button>
        <button className="auth__b" onClick={onSignIn}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="9" /><path d="M12 8h5a5 5 0 1 1-1.5-3" /></svg>Continue with Google</button>
        <button className="auth__b" onClick={onSignIn}><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="5" width="18" height="14" rx="2" /><path d="m3 7 9 6 9-6" /></svg>Continue with email</button>
        <p className="auth__fine">Signing in creates an account the first time.</p>
      </div>
    </div>
  )
}
