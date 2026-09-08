import type { ITerminalOptions } from '@xterm/xterm'

/**
 * How the terminal looks and behaves.
 *
 * Separated from the pane because these are decisions, not wiring: every value
 * below is here for a reason, and a reason belongs next to the number rather
 * than inside a component that also handles resize and teardown.
 */

/**
 * The font stack, longest fallback chain first.
 *
 * Nerd Font entries come before plain `monospace` because a prompt with
 * powerline glyphs in a font that lacks them renders as boxes — and the shells
 * people bring to this terminal are full of them.
 */
const FONT =
  '"SF Mono", "Menlo", "Monaco", "Cascadia Mono", "Consolas", "DejaVu Sans Mono", ' +
  '"Liberation Mono", "Symbols Nerd Font Mono", "MesloLGS Nerd Font", ' +
  '"JetBrainsMono Nerd Font", "Hack Nerd Font", monospace'

/**
 * Rows kept above the top of the screen.
 *
 * Ten thousand is far more than anyone scrolls through and still small enough
 * that a dozen live panes stay in a budget a laptop does not notice.
 */
const SCROLLBACK = 10_000

export function terminalOptions(): ITerminalOptions {
  return {
    // Needed by the webgl and unicode addons; without it they refuse to load.
    allowProposedApi: true,

    cursorBlink: true,
    cursorStyle: 'block',
    // A bar or underline cursor drawn as an outline turns into extra strokes
    // in an unfocused pane. Only a block reads well hollowed out.
    cursorInactiveStyle: 'outline',

    fontFamily: FONT,
    fontSize: 14,
    // Light by default with a heavier bold: the contrast between the two is
    // what makes bold readable, not the absolute weight.
    fontWeight: '300',
    fontWeightBold: '500',
    drawBoldTextInBrightColors: true,

    scrollback: SCROLLBACK,
    // Cells here are taller than a stock terminal's, so a plain one-row wheel
    // step moves less of the screen than the same gesture does elsewhere. The
    // multiplier puts it back where the hand expects it.
    scrollSensitivity: 1.15,
    fastScrollSensitivity: 5,

    // The pane is painted opaque underneath. Transparency here would cost a
    // compositing pass per frame for something nothing shows through.
    allowTransparency: false,

    // On macOS, non-US layouts compose `@` and `€` with Option. Treating it as
    // Meta makes those keys unreachable.
    macOptionIsMeta: false,
    macOptionClickForcesSelection: true,


    theme: {
      background: '#0c0c0e',
      foreground: '#cdcdd4',
      cursor: '#cdcdd4',
      cursorAccent: '#0c0c0e',
      selectionBackground: '#2b3242',

      black: '#15151a',
      red: '#e06c75',
      green: '#8fbf7f',
      yellow: '#d6b26b',
      blue: '#7aa2d6',
      magenta: '#b48ead',
      cyan: '#7fbfbf',
      white: '#cdcdd4',

      brightBlack: '#4b4b57',
      brightRed: '#ef8a92',
      brightGreen: '#a7d199',
      brightYellow: '#e6c98a',
      brightBlue: '#98bce6',
      brightMagenta: '#c9a7c4',
      brightCyan: '#9ad3d3',
      brightWhite: '#eaeaef'
    }
  }
}
