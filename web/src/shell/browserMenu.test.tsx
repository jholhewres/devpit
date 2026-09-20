import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { BrowserMenu } from './BrowserMenu'

/*
 * The panel's own rows, in jsdom.
 *
 * Here and not in the e2e because the panel is a **window of its own** — a
 * pane's page is a second native webview, two native webviews have no z-order
 * between them, and a panel drawn in the pane came out behind the site. The
 * driver hands out one window handle and the menu is not it, so the e2e can
 * only assert what the main window sees: that the page does not move. What the
 * panel says is asserted here.
 */

afterEach(cleanup)

const stores = [
  { family: 'Google Chrome', profile: 'Default', path: '/h/chrome/Default', warning: '' },
  { family: 'Google Chrome', profile: 'Profile 1', path: '/h/chrome/Profile 1', warning: '' },
  { family: 'Firefox', profile: 'work.default', path: '/h/ff/work', warning: 'keyring is asleep' },
]

const imported = vi.fn()

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    browserSessions: () => ['default', 'work'],
    browserSessionHeld: (session: string) => ({ session, bytes: 4096, used: true }),
    browserStores: () => stores,
    browserSessionForget: (session: string) => ({ session, bytes: 0, used: false }),
    browserImport: (pane: string, path: string, domains: string[]) => {
      imported(pane, path, domains)
      return { family: 'Google Chrome · Default', count: 3, domains }
    },
  },
}))

const draw = (over: Partial<Parameters<typeof BrowserMenu>[0]> = {}) =>
  render(
    <BrowserMenu
      pane="p1"
      at="https://github.com/one/two"
      session="default"
      viewport={null}
      granted={false}
      onSession={over.onSession ?? vi.fn()}
      onViewport={over.onViewport ?? vi.fn()}
      onGrant={over.onGrant ?? vi.fn()}
      onSaid={over.onSaid ?? vi.fn()}
      onDone={over.onDone ?? vi.fn()}
      {...over}
    />,
  )

/* The import rows live behind their own flyout — the menu is a cascade, which
   is the shape this was asked for. Opening it is a step every import test
   takes, and forgetting it is how the first draft of this file reported that
   devpit had found no browsers. */
const openImport = async (): Promise<void> => {
  fireEvent.click(screen.getByText('Bring a signed-in session'))
  await waitFor(() => expect(screen.getByText('From Firefox')).toBeTruthy())
}

describe('what the browser menu offers', () => {
  it('lists the sessions with the current one ticked', async () => {
    draw()
    await waitFor(() => expect(screen.getByText('work')).toBeTruthy())
    const current = screen.getByRole('menuitemradio', { name: /default/ })
    expect(current.getAttribute('aria-checked')).toBe('true')
  })

  /* The whole point of the two fields Rust sends: browser first, then which
     one of it. A flat `Google Chrome · Profile 1` list made somebody read the
     browser name five times to find the profile. */
  it('asks for the browser before it asks which profile', async () => {
    draw()
    await openImport()

    /* Chrome has two profiles, so it is a submenu and its profiles are not on
       screen until it is opened. */
    expect(screen.queryByText('Profile 1')).toBeNull()
    fireEvent.click(screen.getByText('From Google Chrome'))
    expect(screen.getByText('Default')).toBeTruthy()
    expect(screen.getByText('Profile 1')).toBeTruthy()

    /* Firefox has one, so it is a row and not a submenu with one thing in it. */
    expect(screen.getByText('From Firefox')).toBeTruthy()
  })

  /* A keyring that will not answer refuses every value at once, which is
     indistinguishable from a corrupt store without this sentence. */
  it('says what stands in the way of a profile before it is picked', async () => {
    draw()
    await openImport()
    expect(screen.getByText('keyring is asleep')).toBeTruthy()
  })

  it('imports nothing until a profile is picked', async () => {
    draw()
    await openImport()
    expect(imported).not.toHaveBeenCalled()
  })

  /* The pane's own host fills the field, so the ordinary import is narrow. */
  it('offers the site you are on, and narrows the import to it', async () => {
    const onSaid = vi.fn()
    draw({ onSaid })
    await openImport()

    const only = screen.getByLabelText('Limit the import to one site') as HTMLInputElement
    expect(only.value).toBe('github.com')

    fireEvent.click(screen.getByText('From Firefox'))
    await waitFor(() => expect(imported).toHaveBeenCalledWith('p1', '/h/ff/work', ['github.com']))
    await waitFor(() => expect(onSaid).toHaveBeenCalled())
    expect(String(onSaid.mock.calls[0][0])).toMatch(/3 cookies/)
  })

  /* Emptying the field is how somebody asks for the profile whole, and the
     row says so rather than leaving it to be discovered. */
  it('takes the profile whole when no site is named, and says so', async () => {
    draw()
    await openImport()

    fireEvent.change(screen.getByLabelText('Limit the import to one site'), {
      target: { value: '' },
    })
    expect(screen.getByText('Every cookie in the profile')).toBeTruthy()
    expect(screen.getByText(/read whole/i)).toBeTruthy()

    fireEvent.click(screen.getByText('From Firefox'))
    await waitFor(() => expect(imported).toHaveBeenCalledWith('p1', '/h/ff/work', []))
  })

  /* Signing out has no undo. It says what it removes, with a size, and it sits
     under a rule of its own rather than one click from the routine import. */
  it('says what signing out costs before it is worth pressing', async () => {
    draw()
    const out = await screen.findByRole('menuitem', { name: /sign out/i })
    expect(out.textContent).toMatch(/4 KB/)
    expect(out.textContent).toMatch(/other sessions are untouched/i)
  })

  /* A width, not a device. Saying otherwise would be claiming a user agent and
     touch emulation this webview cannot do. */
  it('offers page widths and says what a width is not', async () => {
    const onViewport = vi.fn()
    draw({ onViewport })
    fireEvent.click(screen.getByText('Page width'))
    expect(screen.getByText('Mobile M')).toBeTruthy()
    expect(screen.getByText(/user agent and touch support do not change/i)).toBeTruthy()

    fireEvent.click(screen.getByText('Mobile M'))
    expect(onViewport).toHaveBeenCalledWith(expect.objectContaining({ id: 'mobile-m' }))
  })

  /* It was a lit chip in the bar beside a chip that only printed a name. */
  it('carries the agent grant, and says what turning it on means', async () => {
    const onGrant = vi.fn()
    draw({ onGrant })
    const row = await screen.findByRole('menuitemradio', { name: /agent drive this page/i })
    expect(row.textContent).toMatch(/refused, and hears why/i)
    fireEvent.click(row)
    expect(onGrant).toHaveBeenCalledWith(true)
  })

  it('names a new session without creating anything until it is used', async () => {
    const onSession = vi.fn()
    draw({ onSession })
    fireEvent.click(screen.getByText('New session…'))
    fireEvent.change(screen.getByLabelText('Name the session'), { target: { value: 'work2' } })
    fireEvent.click(screen.getByText('Make'))
    expect(onSession).toHaveBeenCalledWith('work2')
  })
})

/*
 * The one thing about this panel that only the stylesheet decides.
 *
 * The flyouts open **leftwards**, and that is not a taste: the panel is
 * anchored to the right of its own window and every pixel of empty room is on
 * its left. A flyout opening rightwards is outside the window, and a window
 * cannot draw outside itself — the submenu would simply be gone.
 *
 * jsdom does no layout, so nothing this file renders could tell. What it can
 * do is read the rule. This exists because the rule was written twice and the
 * second writing did not take: the sheet kept `left: 100%` from when the panel
 * still hung off the toolbar, and nothing anywhere would have said so.
 */
describe('where the flyouts open', () => {
  it('opens them into the room the window has, which is on the left', () => {
    const sheet = readFileSync(resolve(process.cwd(), 'src/shell/styles/24-browser.css'), 'utf8')
    const rule = sheet
      .split('\n')
      .find((line) => /^\.bmenu__sub\s*\{/.test(line.trim()))
    expect(rule, 'the sheet has no rule placing a flyout').toBeTruthy()
    expect(rule).toMatch(/right:\s*100%/)
    expect(rule).not.toMatch(/left:\s*100%/)
  })
})
