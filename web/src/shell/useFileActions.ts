import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import { useState } from 'react'

import { ask, commands } from './live'
import type { ActionId } from './fileMenu'
import { useShell } from './useShell'
import { went } from './problems'
import { changed } from './useTree'

/*
 * What the file context menu's entries actually do, apart from the menu that
 * lists them.
 *
 * Two reasons, not one. The menu is a list and a position on screen; these are
 * three dialogs and three commands, and holding both put `ContextMenu.tsx`
 * past its ceiling. And the split is the honest one: a second surface that
 * wants New file or Delete — the tree's own row actions, say — takes this
 * hook and does not inherit a context menu it has no use for.
 */
export interface FileActions {
  run: (act: ActionId | undefined, path: string | null) => void
  naming: { path: string; folder: boolean } | null
  renaming: string | null
  deleting: string | null
  close: () => void
  create: (name: string) => void
  rename: (name: string) => void
  destroy: () => void
}

/* Where a new entry goes: inside the row when it is a folder, beside it when
   it is a file. `data-kind` is already on every row for the keyboard, so this
   asks the markup rather than threading the node down. */
function parentOf(path: string): string {
  const row = document.querySelector<HTMLElement>(`.row[data-path="${CSS.escape(path)}"]`)
  if (row?.dataset.kind === 'folder') return path
  const cut = path.lastIndexOf('/')
  return cut < 0 ? '' : path.slice(0, cut)
}

export function useFileActions(onActed: () => void): FileActions {
  const shell = useShell()
  const root = shell.project?.rootPath
  const [naming, setNaming] = useState<{ path: string; folder: boolean } | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [deleting, setDeleting] = useState<string | null>(null)

  /* Paths on screen are project-relative; `path.reveal` takes an absolute one
     and checks it against the registered roots itself. */
  const run = (act: ActionId | undefined, path: string | null): void => {
    onActed()
    if (!act || !path) return
    if (act === 'open') {
      /* A folder opens where it is, in the tree; only a file is a tab. */
      const row = document.querySelector<HTMLElement>(`[data-ctx="file"][data-path="${CSS.escape(path)}"]`)
      if (row?.dataset.kind === 'folder') {
        if (row.getAttribute('aria-expanded') !== 'true') row.click()
        return
      }
      shell.show('file', { id: `file:${path}`, path, title: path.split('/').pop() })
      return
    }
    if (act === 'copyPath') {
      /* Through the app: WebKitGTK's own clipboard is not one other programs
         read reliably. */
      void writeText(path).catch(() => went({ error: 'the path could not be copied' }))
      return
    }
    if (act === 'newFile' || act === 'newFolder') {
      setNaming({ path, folder: act === 'newFolder' })
      return
    }
    if (act === 'rename') {
      setRenaming(path)
      return
    }
    if (act === 'delete') {
      setDeleting(path)
      return
    }
    if (root) void ask(() => commands.pathReveal(`${root}/${path}`)).then(went)
  }

  const close = (): void => {
    setNaming(null)
    setRenaming(null)
    setDeleting(null)
  }

  const create = (name: string): void => {
    if (!naming || !shell.project) return
    const parent = parentOf(naming.path)
    const full = parent ? `${parent}/${name}` : name
    const folder = naming.folder
    const project = shell.project.id
    close()
    void ask(() => commands.pathCreate(project, null, full, folder)).then((answer) => {
      went(answer)
      changed()
    })
  }

  /* Rename is `path.move` with both ends in one folder — the backend makes no
     distinction, and neither should this. */
  const rename = (name: string): void => {
    if (!renaming || !shell.project) return
    const cut = renaming.lastIndexOf('/')
    const to = cut < 0 ? name : `${renaming.slice(0, cut)}/${name}`
    const from = renaming
    const project = shell.project.id
    close()
    void ask(() => commands.pathMove(project, null, from, to)).then((answer) => {
      went(answer)
      changed()
    })
  }

  const destroy = (): void => {
    if (!deleting || !shell.project) return
    const path = deleting
    const project = shell.project.id
    close()
    void ask(() => commands.pathDelete(project, null, path)).then((answer) => {
      went(answer)
      changed()
    })
  }

  return { run, naming, renaming, deleting, close, create, rename, destroy }
}
