import { useCallback, useEffect, useReducer } from 'react'

import type { BlockChanged, PaneBlocks } from '../gen/bindings'
import { ask, commands } from './live'
import { changed, EMPTY, happened, loaded, type BlockState } from './paneBlocks'
import { onCarried, onHappening } from './window'

type Action = { kind: 'loaded'; answer: PaneBlocks } | { kind: 'changed'; change: BlockChanged } | { kind: 'happened'; happening: Parameters<typeof happened>[1] }

function reduce(state: BlockState, action: Action): BlockState {
  switch (action.kind) {
    case 'loaded':
      /* Anything heard before the answer came is in the answer too. */
      return loaded(action.answer)
    case 'changed':
      return changed(state, action.change)
    case 'happened':
      return happened(state, action.happening)
  }
}

/** A pane's blocks and where its shell stands, kept current — and a way to
 *  forget the finished ones. */
export function usePaneBlocks(paneId: string): { state: BlockState; clear: () => void } {
  const [state, dispatch] = useReducer(reduce, EMPTY)
  const load = useCallback(
    () =>
      void ask(() => commands.paneBlocks(paneId)).then((answer) => {
        if (answer.data) dispatch({ kind: 'loaded', answer: answer.data })
      }),
    [paneId],
  )
  const clear = useCallback(() => void ask(() => commands.paneBlocksClear(paneId)).then(load), [paneId, load])
  useEffect(() => {
    let live = true
    void ask(() => commands.paneBlocks(paneId)).then((answer) => {
      if (live && answer.data) dispatch({ kind: 'loaded', answer: answer.data })
    })
    const unblock = onCarried<BlockChanged>('terminal:block', (change) => {
      if (change.paneId === paneId) dispatch({ kind: 'changed', change })
    })
    const unhappen = onHappening((happening) => {
      if (happening.paneId === paneId) dispatch({ kind: 'happened', happening })
    })
    return () => {
      live = false
      unblock()
      unhappen()
    }
  }, [paneId])
  return { state, clear }
}
