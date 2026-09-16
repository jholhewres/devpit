/*
 * What a screen has to be for a screenshot of it to mean anything.
 *
 * A PNG proves nothing on its own — a blank window photographs beautifully.
 * Every shot here is taken next to assertions that would fail on the blank
 * one: the landmarks exist by role or label, the container has a size, enough
 * of the picture is not background, nothing overflowed sideways, and the
 * console said nothing.
 */

import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { setTimeout as wait } from 'node:timers/promises'

export const SHOTS = join(process.env.E2E_ROOT ?? '.', 'target/e2e-shots')

/** Starts counting what the page complains about. Call once per window. */
export async function watchTheConsole(window) {
  await window.executeScript(function () {
    window.__e2e = { errors: [], policy: [] }
    const said = console.error.bind(console)
    console.error = function (...args) {
      window.__e2e.errors.push(args.map(String).join(' '))
      said(...args)
    }
    document.addEventListener('securitypolicyviolation', function (event) {
      window.__e2e.policy.push(event.violatedDirective + ' ' + event.blockedURI)
    })
  })
}

export async function complaints(window) {
  return window.executeScript(function () {
    return window.__e2e ?? { errors: [], policy: [] }
  })
}

/** Saves a PNG and answers how much of it is not the background colour. */
export async function shoot(window, name) {
  mkdirSync(SHOTS, { recursive: true })
  const png = await window.takeScreenshot()
  const path = join(SHOTS, `${name}.png`)
  writeFileSync(path, Buffer.from(png, 'base64'))
  return path
}

/**
 * A coarse "is anything drawn here" measure, read from the page rather than
 * from the PNG: the fraction of the window covered by elements that are not
 * the background. Coarse on purpose — a pixel baseline is a test that fails
 * when a font renders differently on another machine, and then gets deleted.
 */
export async function inkFraction(window) {
  return window.executeScript(function () {
    const seen = document.querySelectorAll('button, input, svg, h1, h2, p, span, code, li')
    let covered = 0
    for (const node of seen) {
      const box = node.getBoundingClientRect()
      if (box.width > 0 && box.height > 0) covered += box.width * box.height
    }
    const area = window.innerWidth * window.innerHeight
    return area > 0 ? covered / area : 0
  })
}

/** Nothing sticks out sideways: a window that scrolls horizontally is broken. */
export async function overflowsSideways(window) {
  return window.executeScript(function () {
    return document.documentElement.scrollWidth > document.documentElement.clientWidth + 1
  })
}

/** The shell's own container, and its size. */
export async function shellBox(window) {
  return window.executeScript(function () {
    const app = document.querySelector('.app') ?? document.body
    const box = app.getBoundingClientRect()
    return { width: box.width, height: box.height }
  })
}

/** Gives the window a moment to settle after something was asked of it. */
export async function settle(ms = 400) {
  await wait(ms)
}
