import { useCallback, useEffect, useRef, useState } from 'react'

export const COPIED_MS = 1400

/** Copies text and says so for a moment. The moment ends with the component:
 *  a timer left behind fired after a test's window was gone. */
export function useCopied(ms = COPIED_MS): readonly [boolean, (text: string) => void] {
  const [copied, setCopied] = useState(false)
  const timer = useRef<number | undefined>(undefined)
  const mounted = useRef(true)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
      window.clearTimeout(timer.current)
    }
  }, [])
  const copy = useCallback(
    (text: string) =>
      void navigator.clipboard?.writeText(text).then(() => {
        if (!mounted.current) return
        setCopied(true)
        window.clearTimeout(timer.current)
        timer.current = window.setTimeout(() => setCopied(false), ms)
      }),
    [ms],
  )
  return [copied, copy]
}
