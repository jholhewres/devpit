import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { goToNextChunk, goToPreviousChunk, MergeView, unifiedMergeView } from '@codemirror/merge'
import { EditorState, RangeSetBuilder, type Extension } from '@codemirror/state'
import {
  Decoration,
  drawSelection,
  EditorView,
  keymap,
  lineNumbers,
  ViewPlugin,
  type DecorationSet,
  type ViewUpdate,
} from '@codemirror/view'

import { tokens, type Kind } from './highlight'
import type { Language } from './languages'

/*
 * A file's diff as two editors lined up, or one with the old lines between.
 *
 * `@codemirror/merge` rather than a diff drawn from `git diff`: the whole
 * file is there, unchanged stretches fold away instead of being cut, changes
 * are marked down to the character, and the side on disk can be edited and
 * saved where you are reading it.
 *
 * Coloured by `highlight.ts`, the lexer the file view already trusts, so a
 * file reads the same in both places and nothing here turns text into markup.
 */

export interface Sides {
  /** As `HEAD` holds it; empty for a file `HEAD` does not have. */
  readonly original: string
  /** As it is on disk; empty for a file that was deleted. */
  readonly modified: string
}

export interface Look {
  readonly split: boolean
  /** Unchanged stretches folded down to a few lines of context. */
  readonly folded: boolean
  readonly wrap: boolean
  /** Whether the side on disk takes typing. */
  readonly editable: boolean
}

export interface Merged {
  /** The editor of the side on disk, where typing and navigation happen. */
  readonly editor: EditorView
  destroy(): void
}

const FOLD = { margin: 3, minSize: 4 }

const MARKS: Readonly<Record<Exclude<Kind, 'plain'>, Decoration>> = {
  comment: Decoration.mark({ class: 'tok--comment' }),
  string: Decoration.mark({ class: 'tok--string' }),
  number: Decoration.mark({ class: 'tok--number' }),
  word: Decoration.mark({ class: 'tok--word' }),
  type: Decoration.mark({ class: 'tok--type' }),
  property: Decoration.mark({ class: 'tok--property' }),
}

function coloured(view: EditorView, language: Language | null): DecorationSet {
  const built = new RangeSetBuilder<Decoration>()
  let at = 0
  for (const token of tokens(view.state.doc.toString(), language)) {
    if (token.kind !== 'plain' && token.text.length > 0) {
      built.add(at, at + token.text.length, MARKS[token.kind])
    }
    at += token.text.length
  }
  return built.finish()
}

/** Syntax colour from the in-house lexer, relexed when the text changes. */
export function lexed(language: Language | null): Extension {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet
      constructor(view: EditorView) {
        this.decorations = coloured(view, language)
      }
      update(update: ViewUpdate): void {
        if (update.docChanged) this.decorations = coloured(update.view, language)
      }
    },
    { decorations: (plugin) => plugin.decorations },
  )
}

function common(language: Language | null, look: Look): Extension[] {
  return [
    lineNumbers(),
    drawSelection(),
    history(),
    keymap.of([...defaultKeymap, ...historyKeymap]),
    lexed(language),
    look.wrap ? EditorView.lineWrapping : [],
  ]
}

/** Builds the diff into `parent`, side by side or unified. */
export function mergeInto(
  parent: HTMLElement,
  sides: Sides,
  language: Language | null,
  look: Look,
  changed: (text: string) => void,
): Merged {
  const listen = EditorView.updateListener.of((update) => {
    if (update.docChanged) changed(update.state.doc.toString())
  })
  const onDisk = [
    ...common(language, look),
    EditorState.readOnly.of(!look.editable),
    EditorView.editable.of(look.editable),
    listen,
  ]

  if (look.split) {
    const view = new MergeView({
      parent,
      a: {
        doc: sides.original,
        extensions: [...common(language, look), EditorState.readOnly.of(true), EditorView.editable.of(false)],
      },
      b: { doc: sides.modified, extensions: onDisk },
      highlightChanges: true,
      gutter: true,
      collapseUnchanged: look.folded ? FOLD : undefined,
    })
    return { editor: view.b, destroy: () => view.destroy() }
  }

  const editor = new EditorView({
    parent,
    doc: sides.modified,
    extensions: [
      ...onDisk,
      unifiedMergeView({
        original: sides.original,
        highlightChanges: true,
        gutter: true,
        mergeControls: false,
        collapseUnchanged: look.folded ? FOLD : undefined,
      }),
    ],
  })
  return { editor, destroy: () => editor.destroy() }
}

/** Moves the cursor to the next or the previous change, and into view. */
export function toChange(editor: EditorView, forward: boolean): boolean {
  editor.focus()
  return (forward ? goToNextChunk : goToPreviousChunk)(editor)
}
