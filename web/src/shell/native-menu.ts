/**
 * The browser's own context menu, turned off.
 *
 * A desktop app does not offer "Reload", "Back" or "View page source" on a
 * right-click — those are a browser's answers, and seeing them is the moment a
 * person notices they are looking at a web page in a frame rather than at an
 * application.
 *
 * Editable fields keep theirs: cut, copy and paste with the platform's own
 * shortcuts listed beside them is a real affordance, and replacing it with
 * nothing would take away the only paste some people use.
 *
 * When this app has menus of its own — a right-click on a project, on a file,
 * on a session — they hang off this same handler.
 */
export function suppressNativeMenu(): () => void {
  const onMenu = (event: MouseEvent): void => {
    const target = event.target as HTMLElement | null
    const editable =
      target?.closest('input, textarea, [contenteditable="true"]') !== null && target !== null
    if (!editable) event.preventDefault()
  }

  document.addEventListener('contextmenu', onMenu)
  return () => document.removeEventListener('contextmenu', onMenu)
}
