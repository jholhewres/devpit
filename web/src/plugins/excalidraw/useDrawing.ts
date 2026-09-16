import { usePluginFile, type PluginFile } from '../usePluginFile'
import { drawingFileValid } from './drawingFile'
import { DRAWING, stemOf } from './drawings'

/*
 * One drawing file.
 *
 * Everything here is `usePluginFile`: reading once, writing a moment after the
 * last change, refusing a write over a file somebody else changed. What is
 * Excalidraw's own is the one rule below — a canvas opened on a file that is
 * not a drawing would save an empty scene over it.
 */

export type DrawingFile = PluginFile

export function useDrawing(name: string): DrawingFile {
  return usePluginFile(DRAWING, name, (text) =>
    drawingFileValid(text)
      ? null
      : `${stemOf(name)} cannot be opened here, so it is left as it is: it is not an Excalidraw drawing, or it embeds web content.`,
  )
}
