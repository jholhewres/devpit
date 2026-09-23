import { useCallback, useEffect, useMemo, useState } from 'react'

import { ask, commands } from './live'
import { inOrder } from './inOrder'
import { fits, WIDE, type Panel, type Widths } from './sizing'
import { onResized } from './window'

/*
 * How wide the two side panels are.
 *
 * Its own file because it is three things that belong together and nowhere
 * else: the width while a divider is being dragged, the write that happens
 * once it is let go, and the squeeze when the window itself gets smaller.
 */

export interface Sizing {
  readonly widths: Widths
  /** While a divider is being dragged: paints, does not write. */
  setWidth: (panel: Panel, wide: number) => void
  /** When it is let go: paints and writes. */
  keepWidths: (panel: Panel, wide: number) => void
}

export function useWidths(): Sizing {
  const [widths, setWidths] = useState<Widths>(WIDE)

  /* Read back once, so the window opens the way it was left. */
  useEffect(() => {
    void ask(() => commands.panelWidths()).then((asked) => {
      if (asked.data) setWidths(fits(asked.data, window.innerWidth))
    })
  }, [])

  /*
   * A window made narrower has to take the room from somewhere, and the grid
   * takes it from the middle — down to nothing, because a grid does not
   * argue. Held here instead, so the panels give it back and the work stays
   * on screen.
   *
   * Not written: this is the window being resized, not a choice, and
   * remembering it would mean a person who briefly narrowed their window
   * comes back to panels they never touched.
   */
  useEffect(
    () => onResized(() => setWidths((was) => fits(was, window.innerWidth))),
    [],
  )

  const setWidth = useCallback(
    (panel: Panel, wide: number) => setWidths((was) => ({ ...was, [panel]: wide })),
    [],
  )

  const keepWidths = useCallback((panel: Panel, wide: number) => {
    setWidths((was) => {
      const next = { ...was, [panel]: wide }
      /* On the drop and never during the drag: a preference row rewritten on
         every pointer move is a disk write per frame for a number nobody
         reads until the next launch. */
      void inOrder('widths', () => ask(() => commands.panelWidthsWrite(next.sidebar, next.files)))
      return next
    })
  }, [])

  return useMemo(() => ({ widths, setWidth, keepWidths }), [widths, setWidth, keepWidths])
}
