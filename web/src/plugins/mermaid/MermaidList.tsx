import { paneMeta } from '../../shell/paneList'
import type { Tab } from '../../shell/strip'
import { PluginFileList } from '../PluginFileList'
import { MERMAID } from './mermaidFiles'

/** The project's diagrams. Everything here is the shared list. */
export function MermaidList({ tab }: { tab: Tab }): React.JSX.Element {
  return (
    <PluginFileList
      kind={MERMAID}
      tab={tab}
      pane="diagram"
      title="Mermaid"
      icon={paneMeta('diagram').icon}
      note={
        <>
          Diagrams written as text, kept as <code>.mmd</code> files in this project&rsquo;s folder.
        </>
      }
    />
  )
}
