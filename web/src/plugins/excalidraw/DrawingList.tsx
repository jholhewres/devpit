import { paneMeta } from '../../shell/paneList'
import type { Tab } from '../../shell/strip'
import { PluginFileList } from '../PluginFileList'
import { DRAWING } from './drawings'

/** The project's drawings. Everything here is the shared list. */
export function DrawingList({ tab }: { tab: Tab }): React.JSX.Element {
  return (
    <PluginFileList
      kind={DRAWING}
      tab={tab}
      pane="drawing"
      title="Excalidraw"
      icon={paneMeta('drawing').icon}
      note={
        <>
          Excalidraw drawings, kept as <code>.excalidraw</code> files in this project&rsquo;s
          folder.
        </>
      }
    />
  )
}
