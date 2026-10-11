/* The cheat sheet opens on its key, or from somewhere that is not a key — the
   palette's row — through this event, which `CheatSheet` listens for. */

export const KEYS_SHEET = 'devpit:keys-sheet'

export const openKeysSheet = (): void => void window.dispatchEvent(new Event(KEYS_SHEET))
