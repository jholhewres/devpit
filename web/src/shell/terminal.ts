import type { ITerminalOptions, ITheme } from '@xterm/xterm'

/*
 * What a terminal looks like, and how it behaves.
 *
 * Apart from the component because these are decisions, not wiring: which
 * sixteen colours a program gets to paint with, how fast a wheel scrolls, how
 * far back the history goes. A component that also held them would hide them.
 *
 * The palette is Ghostty's, by way of Orca. Adopting it wholesale rather than
 * inventing one is the point: every CLI worth using was tuned against a
 * palette in this family, and a bespoke sixteen would make `git diff` and a
 * spinner and an error look subtly wrong in ways nobody could name.
 */

/* Ghostty Default Dark, with one change carried over from Orca: the original
   selection blue (#3e4451) disappears into the grey instruction blocks the
   agent CLIs print, so the selection is lifted until it reads as one. */
const DARK: ITheme = {
  background: '#282c34',
  foreground: '#e2e2e2',
  cursor: '#e2e2e2',
  cursorAccent: '#282c34',
  selectionBackground: '#5a7898',
  selectionForeground: '#ffffff',
  black: '#1d1f21',
  red: '#cc6666',
  green: '#b5bd68',
  yellow: '#f0c674',
  blue: '#81a2be',
  magenta: '#b294bb',
  cyan: '#8abeb7',
  white: '#c5c8c6',
  brightBlack: '#666666',
  brightRed: '#d54e53',
  brightGreen: '#b9ca4a',
  brightYellow: '#e7c547',
  brightBlue: '#7aa6da',
  brightMagenta: '#c397d8',
  brightCyan: '#70c0b1',
  brightWhite: '#eaeaea',
}

/* Tango, with the near-white values darkened. The agent CLIs use the ANSI
   accents for text rather than for decoration, so Tango's legacy yellow and
   cyan would be a sentence you cannot read on white. */
const LIGHT: ITheme = {
  background: '#ffffff',
  foreground: '#2e3434',
  cursor: '#2e3434',
  cursorAccent: '#ffffff',
  selectionBackground: '#accef7',
  selectionForeground: '#2e3434',
  black: '#2e3436',
  red: '#cc0000',
  green: '#4e9a06',
  yellow: '#8e7700',
  blue: '#3465a4',
  magenta: '#75507b',
  cyan: '#05727e',
  white: '#6a6a6a',
  brightBlack: '#555753',
  brightRed: '#ef2929',
  brightGreen: '#1b7a1b',
  brightYellow: '#6d5a00',
  brightBlue: '#204a87',
  brightMagenta: '#ad7fa8',
  brightCyan: '#034b50',
  brightWhite: '#3d3d3d',
}

export const palette = (dark: boolean): ITheme => (dark ? DARK : LIGHT)

/* A terminal that keeps five thousand rows is one you can scroll back through
   after a build; one that keeps the default thousand is one that has already
   thrown away the error you went looking for. */
const SCROLLBACK = 5000

/* The contrast colours are lifted to. The person's choice when there is one;
   otherwise the old rule — none on dark, 4.5 on light, because a program that
   paints mid-grey is unreadable on white and fine on black. */
export function contrastFor(dark: boolean, chosen: number | null): number {
  return chosen ?? (dark ? 1 : 4.5)
}

export function options(dark: boolean, contrast: number | null = null): ITerminalOptions {
  return {
    allowProposedApi: true,
    theme: palette(dark),

    /* A chain rather than one family: a Nerd Font is what draws the glyphs in
       an agent's spinner and a powerline prompt, and whichever of these the
       machine has is the one that will.
    
       Geist Mono is the window's font and it is not this one. It is a variable
       webfont, and xterm measures a cell from the font it is given: at weight
       300 the variable axis draws thinner than the metrics the atlas was built
       from, which is a terminal that looks washed out next to the same shell
       in any other window. System monospace first, as Orca does. */
    fontFamily:
      '"SF Mono", "Menlo", "Monaco", "Cascadia Mono", "Consolas", "DejaVu Sans Mono", "Liberation Mono", "Symbols Nerd Font Mono", "MesloLGS Nerd Font", "JetBrainsMono Nerd Font", monospace',
    fontSize: 14,
    /* Light, with bold a step up rather than a jump. A TUI uses bold for
       emphasis on nearly every line, so the gap has to be small enough that a
       screen of it still reads as text. */
    fontWeight: '300',
    fontWeightBold: '500',
    /* 1.0: a terminal's rows are the program's to space, and a fifth of a line
       added to each one is a screen that fits four fewer rows than the TUI
       drawing it expects. */
    lineHeight: 1.0,

    cursorBlink: true,
    cursorStyle: 'block',
    /* An unfocused block cursor becomes an outline. A bar or underline would
       only gain a second stroke, which reads as two cursors. */
    cursorInactiveStyle: 'outline',

    scrollback: SCROLLBACK,
    /* Cells here are taller than most terminals', so a wheel notch moving the
       same number of rows would move further down the page than the hand
       expects. */
    scrollSensitivity: 1.15,
    fastScrollSensitivity: 5,
    /* No `scrollbar` width here: the option is in xterm's beta line and this
       is the stable one, and a cast to reach it would be claiming a version
       we do not have. */

    /* Bold text takes the bright colour, which is what every CLI is written
       against — without it a bold red error is the same red as ordinary
       output. */
    drawBoldTextInBrightColors: true,
    /* Only on the light ground: a program that paints mid-grey on white is
       unreadable there and fine on black, so lifting contrast everywhere
       would flatten a palette that already works. */
    minimumContrastRatio: contrastFor(dark, contrast),

    allowTransparency: false,
    /* On a non-US layout Option composes `@` and `€`. Treating it as Meta
       would cost the person those characters. */
    macOptionIsMeta: false,
    macOptionClickForcesSelection: true,
  }
}

/* Whether the window is dark right now.
 *
 * The app stamps its choice on the root element; `system` stamps nothing and
 * leaves the question to the OS. */
export function darkNow(): boolean {
  if (typeof document === 'undefined') return true
  const chosen = document.documentElement.dataset.theme
  if (chosen === 'dark') return true
  if (chosen === 'light') return false
  return !window.matchMedia('(prefers-color-scheme: light)').matches
}
