/*
 * What removing a project does to the list, and to where you are standing.
 *
 * A function rather than lines inside the hook, so the test can call it: the
 * case that matters is removing the project you are *in*, and a test that
 * restates "it lands on another one" passes whether or not the window still
 * does that.
 */
export interface Open {
  readonly projects: readonly string[]
  readonly current: string
}

export function forgotten(open: Open, name: string): Open {
  const projects = open.projects.filter((other) => other !== name)
  return {
    projects,
    /* Removing the ground you are standing on has to land somewhere, or the
       window keeps naming a project that is no longer listed. */
    current: open.current === name ? (projects[0] ?? '') : open.current,
  }
}
