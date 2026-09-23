/* A ref for the highlighted row of a list walked with the arrows: brought
   into view as it becomes the one, so the keyboard never walks off the edge
   of what is shown. `nearest`, so a row already visible does not jump. */
export const intoView = (row: HTMLElement | null): void => {
  row?.scrollIntoView?.({ block: 'nearest' })
}
