import { paneMeta } from '../../shell/paneList'
import type { Tab } from '../../shell/strip'
import { PluginFileList } from '../PluginFileList'
import { NOTES_KIND } from './noteFiles'

/** The project's notes. Everything here is the shared list. */
export function NotesList({ tab }: { tab: Tab }): React.JSX.Element {
  return (
    <PluginFileList
      kind={NOTES_KIND}
      tab={tab}
      pane="note"
      title="Notes"
      icon={paneMeta('note').icon}
      note={
        <>
          Notes in Markdown, kept as <code>.md</code> files in this project&rsquo;s folder.
        </>
      }
    />
  )
}
