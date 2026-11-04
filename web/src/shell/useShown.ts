import { useEffect, useRef, useState } from 'react'

/* How many cards a lane draws first, and adds each time its end comes into view. */
export const LANE_PAGE = 30

/* How far below the view the next cards are drawn, in pixels. */
const AHEAD = 400

/* How many of a lane's `total` cards to draw: a page, and another each time
   the mark after the last one comes near the view. Every card stays in the
   lane's data, so a search and a drop still reach the ones not drawn yet.
   `box` is the lane's own scroller, which the mark scrolls in. */
export function useShown(
  total: number,
  box: React.RefObject<HTMLElement | null>,
): { shown: number; mark: React.RefObject<HTMLDivElement | null> } {
  const [shown, setShown] = useState(LANE_PAGE)
  const mark = useRef<HTMLDivElement>(null)
  const more = shown < total

  useEffect(() => {
    const at = mark.current
    if (!at || !more || typeof IntersectionObserver === 'undefined') return
    const watch = new IntersectionObserver(
      (seen) => seen.some((one) => one.isIntersecting) && setShown((was) => was + LANE_PAGE),
      { root: box.current, rootMargin: `0px 0px ${AHEAD}px 0px` },
    )
    watch.observe(at)
    return () => watch.disconnect()
  }, [more, shown, box])

  return { shown: Math.min(shown, total), mark }
}
