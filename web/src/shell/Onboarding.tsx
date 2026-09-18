import { useEffect, useState } from 'react'

import mark from '../assets/brand/mark.png'
import { OnboardingAccount } from './OnboardingAccount'
import { useShell } from './useShell'

/*
 * The first run, and the empty state — the same screen, because they are the
 * same situation: devpit with no project is devpit with nothing to show.
 *
 * Three steps, and the first is optional and says what an account is worth
 * today: a name, not sync. A first screen that reads as a wall is one people
 * close, and one that promises what is not built is worse.
 *
 * Signing in already? Then that step is not asked again. The empty state is
 * reachable long after the first run, and asking a signed-in person to sign
 * in is the app admitting it was not paying attention.
 */
export function Onboarding({
  onAddProject,
  onDone,
}: {
  onAddProject: () => void
  onDone: () => void
}): React.JSX.Element {
  const { signedIn, theme, setTheme } = useShell()
  const [step, setStep] = useState(signedIn ? 1 : 0)

  /* The approval happens in the browser and lands here, so the step it lands
     on is the one that moves. It used to advance on the click that opened the
     browser: the code was never on screen, and whoever approved it came back
     to a theme picker with no idea whether it had worked. */
  useEffect(() => {
    if (signedIn) setStep((now) => (now === 0 ? 1 : now))
  }, [signedIn])

  const steps = ['Account', 'Theme', 'Project'] as const

  return (
    <div className="onb">
      <div className="onb__box">
        <span className="onb__mark">
          <img className="mark" alt="" src={mark} />
        </span>

        <ol className="onb__rail" aria-label="Setup">
          {steps.map((name, index) => (
            <li key={name} className="onb__dot" data-state={index === step ? 'now' : index < step ? 'done' : 'next'}>
              <span>{name}</span>
            </li>
          ))}
        </ol>

        {step === 0 && <OnboardingAccount onSkip={() => setStep(1)} />}

        {step === 1 && (
          <>
            <h1 className="onb__t">Pick a theme</h1>
            <p className="onb__d">You can change it later in Settings.</p>
            <div className="thm" role="radiogroup" aria-label="Theme">
              {(['system', 'light', 'dark'] as const).map((name) => (
                <button
                  key={name}
                  className="thm__c"
                  role="radio"
                  aria-checked={theme === name}
                  onClick={() => setTheme(name)}
                >
                  <span className={`thm__p thm__p--${name === 'system' ? 'sys' : name}`}>
                    {name === 'system' ? (
                      <>
                        <span className="thm__lay thm__p--light">
                          <span className="thm__side" />
                          <span className="thm__main">
                            <span className="thm__bar" />
                            <span className="thm__bar thm__bar--s" />
                            <span className="thm__bar" />
                          </span>
                        </span>
                        <span className="thm__lay thm__p--dark">
                          <span className="thm__side" />
                          <span className="thm__main">
                            <span className="thm__bar" />
                            <span className="thm__bar thm__bar--s" />
                            <span className="thm__bar" />
                          </span>
                        </span>
                      </>
                    ) : (
                      <>
                        <span className="thm__side" />
                        <span className="thm__main">
                          <span className="thm__bar" />
                          <span className="thm__bar thm__bar--s" />
                          <span className="thm__bar" />
                        </span>
                      </>
                    )}
                  </span>
                  <span className="thm__l">
                    {name === 'system' ? 'System' : name === 'light' ? 'Light' : 'Dark'}
                  </span>
                </button>
              ))}
            </div>
            <button className="onb__go" onClick={() => setStep(2)}>
              Continue
            </button>
            <button className="onb__skip" onClick={() => setStep(0)}>
              Back
            </button>
          </>
        )}

        {step === 2 && (
          <>
            <h1 className="onb__t">Open a project</h1>
            <p className="onb__d">
              A folder with a git repository in it. devpit reads it where it is and never moves it.
            </p>
            <button
              className="onb__go"
              onClick={() => {
                onDone()
                onAddProject()
              }}
            >
              Add a project
            </button>
            <button className="onb__skip" onClick={() => setStep(1)}>
              Back
            </button>
          </>
        )}
      </div>
    </div>
  )
}
