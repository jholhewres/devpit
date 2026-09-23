import { act, cleanup, fireEvent, render } from '@testing-library/react'
import { useRef } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { useRailDrag, type Grab, type Spot } from './useRailDrag'

afterEach(cleanup)

function Rail({ onDrop, onClick }: { onDrop: (grab: Grab, spot: Spot) => void; onClick: () => void }): React.JSX.Element {
  const list = useRef<HTMLDivElement>(null)
  const drag = useRailDrag(list, onDrop)
  return (
    <div ref={list}>
      <button data-drop="project" data-key="a" onPointerDown={drag.press({ kind: 'project', id: 'a' })} onClick={() => drag.clicked() && onClick()}>
        a
      </button>
      <button data-drop="group" data-key="Work">
        Work
      </button>
    </div>
  )
}

const pointer = (type: string, x: number, y: number): Event =>
  Object.assign(new Event(type, { bubbles: true, cancelable: true }), { clientX: x, clientY: y, button: 0 })

describe('dragging in the rail', () => {
  it('drops a project on the row under the pointer, on the side the pointer is', () => {
    const onDrop = vi.fn()
    const onClick = vi.fn()
    const { getByText } = render(<Rail onDrop={onDrop} onClick={onClick} />)
    const heading = getByText('Work')
    heading.getBoundingClientRect = () => ({ top: 100, height: 20, bottom: 120, left: 0, right: 0, width: 0, x: 0, y: 100, toJSON: () => ({}) })
    document.elementFromPoint = () => heading

    fireEvent.pointerDown(getByText('a'), { button: 0, clientX: 10, clientY: 10 })
    act(() => {
      window.dispatchEvent(pointer('pointermove', 10, 60))
      window.dispatchEvent(pointer('pointermove', 10, 115))
      window.dispatchEvent(pointer('pointerup', 10, 115))
    })
    expect(onDrop).toHaveBeenCalledWith({ kind: 'project', id: 'a' }, { kind: 'group', key: 'Work', after: true })
    // The click the release makes is the drop's.
    fireEvent.click(getByText('a'))
    expect(onClick).not.toHaveBeenCalled()
  })

  it('is still a click when the pointer barely moved', () => {
    const onDrop = vi.fn()
    const onClick = vi.fn()
    const { getByText } = render(<Rail onDrop={onDrop} onClick={onClick} />)
    fireEvent.pointerDown(getByText('a'), { button: 0, clientX: 10, clientY: 10 })
    act(() => {
      window.dispatchEvent(pointer('pointermove', 12, 11))
      window.dispatchEvent(pointer('pointerup', 12, 11))
    })
    fireEvent.click(getByText('a'))
    expect(onDrop).not.toHaveBeenCalled()
    expect(onClick).toHaveBeenCalledOnce()
  })
})
