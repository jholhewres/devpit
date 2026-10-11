import { useEffect, useState } from 'react'

import { isMaximized, onResized } from './window'

/** Whether the window fills the screen, kept current as it is resized. */
export function useMaximized(): boolean {
  const [maximized, setMaximized] = useState(false)
  useEffect(() => {
    const check = (): void => void isMaximized().then(setMaximized)
    check()
    return onResized(check)
  }, [])
  return maximized
}
