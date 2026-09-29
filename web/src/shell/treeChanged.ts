/* What a mutation outside the tree hook announces when it has changed the
   tree. An event, not a callback threaded through four components: the panel
   that owns the tree and the menu that creates a file are siblings, and the
   menu has no business holding a reference to the panel's reload. */
export const CHANGED = 'devpit:tree-changed'

export const changed = (): void => {
  window.dispatchEvent(new Event(CHANGED))
}
