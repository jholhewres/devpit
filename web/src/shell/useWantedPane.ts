import { useEffect, useState } from 'react'

import { tabOfPane, type Tab } from './strip'

/*
 * A terminal pane asked for from another project's view: its tabs arrive
 * after the switch, and the one holding the pane is focused then.
 */
export function useWantedPane(open: readonly Tab[], focus: (id: string) => void): (paneId: string | null) => void {
  const [wanted, setWanted] = useState<string | null>(null)
  useEffect(() => {
    if (!wanted) return
    const tab = tabOfPane(open, wanted)
    if (!tab) return
    focus(tab)
    setWanted(null)
  }, [wanted, open, focus])
  return setWanted
}
