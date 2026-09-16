import { paneMeta } from '../../shell/paneList'
import type { Tab } from '../../shell/strip'
import { PluginFileList } from '../PluginFileList'
import { DATA } from './dataFiles'

/** The project's data files. Everything here is the shared list. */
export function DataList({ tab }: { tab: Tab }): React.JSX.Element {
  return (
    <PluginFileList
      kind={DATA}
      tab={tab}
      pane="data"
      title="Data"
      icon={paneMeta('data').icon}
      note={
        <>
          JSON and YAML, kept as <code>.json</code> and <code>.yaml</code> files in this
          project&rsquo;s folder, and drawn as a graph you can walk.
        </>
      }
    />
  )
}
