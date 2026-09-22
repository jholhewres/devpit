import { useCallback, useEffect, useRef, useState } from 'react'

import type { LayoutNode } from '../gen/bindings'
import { ask, commands } from './live'
import { Leaf } from './Leaf'
import { usePaneActions } from './paneActions'
import { PaneCorner } from './PaneCorner'
import { Split } from './Split'
import { shortcutFor } from './shortcuts'
import { leaves } from './splits'
import type { Tab } from './strip'
import { useShell } from './useShell'

/*
 * One tab's terminals, arranged as its tree says.
 *
 * The tab owns the tree. It used to be one tree per project, which meant a
 * second terminal tab had to split the first one's to get a leaf of its own —
 * so three tabs were three leaves under two split nodes, drawn as three tabs.
 * A list wearing a tree's clothes. Now a tab is a tree, and splitting divides
 * what one tab shows, which is what the word means.
 */

export function TerminalPane({ tab, projectId }: { tab: Tab; projectId: string }): React.JSX.Element {
  const { attach, launched } = useShell()
  const [tree, setTree] = useState<LayoutNode | null>(null)
  const [focused, setFocused] = useState('')
  const [error, setError] = useState<string | null>(null)
  /* Something went wrong beside the terminal rather than instead of it. */
  const [notice, setNotice] = useState<string | null>(null)

  useEffect(() => {
    if (!projectId) return
    let dropped = false
    void ask(() => commands.sessionEnsure(projectId, tab.id, null)).then((answer) => {
      if (dropped) return
      if (!answer.data) return setError(answer.error ?? 'could not open a terminal')
      setTree(answer.data.tree)
      setFocused(answer.data.focusedId)
    })
    return () => {
      dropped = true
    }
  }, [projectId, tab.id])

  const split = useCallback(
    (direction: 'horizontal' | 'vertical') => {
      if (!projectId || !focused) return
      void ask(() => commands.sessionSplit(projectId, tab.id, focused, direction, null)).then(
        (answer) => {
          if (!answer.data) return setError(answer.error ?? 'could not split')
          setTree(answer.data.tree)
          setFocused(answer.data.focusedId)
        },
      )
    },
    [projectId, tab.id, focused],
  )

  /* The shortcuts every terminal with splits uses. Only while this tab is the
     one in front, or a hidden tab would split on a key meant for the visible
     one. */
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      const press = shortcutFor(event)
      if (press !== 'splitRight' && press !== 'splitDown') return
      event.preventDefault()
      split(press === 'splitRight' ? 'horizontal' : 'vertical')
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [split])

  const settle = useCallback(
    (splitId: string, ratio: number) => {
      if (!projectId) return
      void ask(() => commands.sessionSetRatio(projectId, tab.id, splitId, ratio)).then(
        (answer) => {
          if (answer.data) setTree(answer.data.tree)
        },
      )
    },
    [projectId, tab.id],
  )

  const focus = useCallback(
    (leafId: string) => {
      setFocused((was) => {
        if (was === leafId || !projectId) return was
        void ask(() => commands.sessionFocus(projectId, tab.id, leafId))
        return leafId
      })
    },
    [projectId, tab.id],
  )

  /* The tab is told which leaves it is showing, because the strip and the
     sidebar draw this tab and neither can see the tree. Nothing ever told
     them, so a terminal with an agent open looked exactly like an empty one
     in both places. */
  useEffect(() => {
    if (tree) attach(tab.id, leaves(tree))
  }, [tree, tab.id, attach])

  /* A terminal opened to run an agent runs it once its pane exists.

     Both a ref and the tab's own state, and neither alone is enough: the
     state is what stops a later render from sending it again, and the ref is
     what stops the second of React's two development mounts from sending it
     before that state has landed. Two sends is two `claude` lines typed, the
     second one into the first one. */
  const sent = useRef(false)
  useEffect(() => {
    if (!projectId || !focused || !tab.launch || sent.current) return
    sent.current = true
    const agent = tab.launch
    launched(tab.id)
    void ask(() => commands.sessionLaunchAgent(projectId, focused, agent)).then((answer) => {
      /* A notice, not the fatal error. The terminal is open and working
         whether or not the agent was started for you, and replacing a working
         terminal with a sentence takes away the one thing that still lets you
         type the command yourself. */
      if (answer.error) setNotice(answer.error)
    })
  }, [projectId, focused, tab.launch, tab.id, launched])

  const layoutTo = useCallback((next: LayoutNode, focusedId: string) => {
    setTree(next)
    setFocused(focusedId)
  }, [])
  const pane = usePaneActions({ tab, tree, focused, onLayout: layoutTo, onNotice: setNotice })

  if (error) return <div className="exempty__t">{error}</div>
  if (!tree) return <div className="termhost" />

  return (
    <>
      <PaneCorner
        tabId={tab.id}
        what="terminal"
        onSplit={split}
        onChat={pane.toChat ?? undefined}
        onClose={pane.closePane}
        closeLabel={pane.closeLabel}
        closeArmed={pane.armed}
      />
      {notice && (
        <button className="tnote" onClick={() => setNotice(null)} title="Dismiss">
          {notice}
        </button>
      )}
      <Split
        node={tree}
        focused={focused}
        onFocus={focus}
        onRatio={settle}
        leaf={(leafId) => <Leaf key={leafId} paneId={leafId} projectId={projectId} />}
      />
    </>
  )
}
