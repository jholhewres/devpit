import { Grip } from './Grip'
import { useShell } from './useShell'

/*
 * The two edges you can drag.
 *
 * Drawn last and positioned rather than placed in the grid: an edge that were
 * a column of its own would be a column the layout has to make room for, and
 * it is meant to sit on the seam.
 *
 * A hidden panel has no edge. Dragging the boundary of something that is not
 * on screen would be resizing a thing nobody can see.
 */
export function Grips(): React.JSX.Element {
  const { side, files, widths, setWidth, keepWidths } = useShell()

  return (
    <>
      {side && (
        <Grip
          panel="sidebar"
          width={widths.sidebar}
          other={files ? widths.files : 0}
          onSize={(wide) => setWidth('sidebar', wide)}
          onDone={(wide) => keepWidths('sidebar', wide)}
        />
      )}
      {files && (
        <Grip
          panel="files"
          width={widths.files}
          other={side ? widths.sidebar : 0}
          onSize={(wide) => setWidth('files', wide)}
          onDone={(wide) => keepWidths('files', wide)}
        />
      )}
    </>
  )
}
