import { cleanup, render, screen } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { Onboarding } from './Onboarding'

afterEach(cleanup)

vi.mock('./useShell', () => ({
  useShell: () => ({ signedIn: false, signIn: vi.fn(), theme: 'dark', setTheme: vi.fn() }),
}))

/*
 * What the app tells somebody it does, pinned.
 *
 * The first screen said an account "saves the workspace around your work —
 * your board, the capabilities you turn on, your theme". None of that is
 * built: signing in fetches a name and an address and stops there. Copy is
 * not a detail here — somebody who believed it would have no backup of the
 * board they think is saved.
 */
describe('what the first run promises', () => {
  it('offers the account without promising sync', () => {
    render(<Onboarding onAddProject={vi.fn()} onDone={vi.fn()} />)
    const said = screen.getByText(/Free, and optional/).textContent ?? ''
    expect(said).toMatch(/not built yet/)
    expect(said).not.toMatch(/saves/)
    expect(said).not.toMatch(/board|capabilit|theme/i)
  })
})

describe('what the settings screen promises', () => {
  const settings = readFileSync(resolve(process.cwd(), 'src/shell/Settings.tsx'), 'utf8')

  it('says what the account does today, and that sync is not built', () => {
    expect(settings).toMatch(/nothing is uploaded and nothing is synced/i)
    expect(settings).toMatch(/sync between machines is not built yet/i)
  })

  /* A switch for something nothing reads is worse than no switch: it is a
     choice the person makes and the app ignores. */
  it('has no switch for anything that does not happen', () => {
    expect(settings).not.toMatch(/telemetry/i)
    expect(settings).not.toMatch(/keepTranscripts/)
    expect(settings).not.toMatch(/anonymous usage/i)
  })
})
