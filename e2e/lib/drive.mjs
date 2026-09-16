/*
 * What a person does to the window, said the way they would say it.
 *
 * Clicks go through the DOM where a real click would be intercepted by an
 * overlay a person would not be fighting — the confirm dialogs and the menus
 * are portals over everything, and the button under them is not what anybody
 * is aiming at.
 */

import { By } from 'selenium-webdriver'
import { setTimeout as wait } from 'node:timers/promises'

export const settle = (ms = 400) => wait(ms)

/** Clicks the visible button whose words are exactly these. */
export async function press(window, words) {
  const pressed = await window.executeScript(function (words) {
    const buttons = Array.prototype.slice.call(document.querySelectorAll('button'))
    const hit = buttons.find(function (node) {
      const said = (node.getAttribute('aria-label') || node.innerText || '').trim()
      return said === words && node.offsetParent !== null
    })
    if (!hit) return false
    hit.click()
    return true
  }, words)
  if (!pressed) throw new Error(`no visible button says "${words}"`)
  await settle()
}

/** Right-clicks the tile with this title, and waits for its menu. */
export async function openCardMenu(window, title) {
  // Whatever was open is closed first: an open card or a menu over the board
  // takes the right-click, and the tile never hears it. Dispatched rather
  // than typed — with a terminal focused, the body is not "interactable".
  await escape(window)
  const menu = By.css(`[role="menu"][aria-label="${title} actions"]`)
  for (let attempt = 0; attempt < 3; attempt += 1) {
    // Dispatched rather than right-clicked through the driver: the driver
    // refuses an element it judges covered, and what covers a tile after a
    // terminal was in front is a layer that is already on its way out.
    await window.executeScript(function (title) {
      const tiles = Array.prototype.slice.call(document.querySelectorAll('[data-card]'))
      const tile = tiles.find(function (one) {
        return one.innerText.split('\n').some(function (line) {
          return line.trim() === title
        })
      })
      if (!tile) return
      tile.scrollIntoView({ block: 'center' })
      const box = tile.getBoundingClientRect()
      tile.dispatchEvent(
        new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          clientX: box.left + box.width / 2,
          clientY: box.top + box.height / 2,
        }),
      )
    }, title)
    await settle(400)
    if ((await window.findElements(menu)).length > 0) return
  }
  throw new Error(`the menu for "${title}" did not open`)
}

/** Escape, the way the window hears it, whatever has focus. */
export async function escape(window) {
  await window.executeScript(function () {
    const target = document.activeElement ?? document.body
    target.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
  })
  await settle(300)
}

/** Opens a lane's own menu by its name. */
export async function openLaneMenu(window, lane) {
  await press(window, `${lane} actions`)
}

/**
 * Puts words in a field and presses Enter, the way typing would have.
 *
 * Not `sendKeys` for the words: WebKitWebDriver dispatches the keydown events
 * and inserts nothing — the field keeps its old value while the page sees
 * every key go by. So the value is set through the element's own setter, with
 * the `input` event React listens for, and only Enter is sent as a key, which
 * is the part the app actually reacts to.
 */
export async function fill(window, selector, words) {
  const field = await window.findElement(By.css(selector))
  await window.executeScript(
    function (field, words) {
      field.focus()
      if (field.isContentEditable) {
        field.textContent = words
      } else {
        const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), 'value').set
        setter.call(field, words)
      }
      field.dispatchEvent(new Event('input', { bubbles: true }))
    },
    field,
    words,
  )
  // Enter dispatched too: a textarea that has only just mounted is "not
  // focusable" to the driver for a moment, and the keydown is all the app
  // listens for anyway.
  await settle(200)
  await window.executeScript(function (field) {
    field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true }))
  }, field)
  await settle(700)
}

export async function text(window) {
  return (await window.executeScript('return document.body.innerText')).replace(/\s+/g, ' ')
}
