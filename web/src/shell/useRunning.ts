import { useCallback, useEffect, useState } from "react";

import type { PaneRunning } from "../gen/bindings";
import { ask, commands } from "./live";
import { same } from "./running";
import { whileWatched } from "./whileWatched";
import { inTauri, onCarried } from "./window";

/*
 * What each of a project's panes has in the foreground.
 *
 * The window asks once, then again when the app says `panes:changed` for the
 * project: the app reads tmux and the process table once for every caller,
 * and only while the window is on screen. A page with no app events behind it
 * asks on a timer instead.
 */

const EVERY_MS = 2000;

export function useRunning(projectId: string | null): readonly PaneRunning[] {
  const [running, setRunning] = useState<readonly PaneRunning[]>([]);

  const look = useCallback(() => {
    if (!projectId) {
      setRunning((was) => (was.length === 0 ? was : []));
      return;
    }
    return ask(() => commands.sessionRunning(projectId)).then((answer) => {
      /* A project with no session yet answers with an empty list rather
         than an error, so there is nothing here to report. Every answer is
         a fresh array and the shell's context is rebuilt on a new one, so
         the old one is kept while nothing changed — otherwise everything
         under it redraws every two seconds. */
      const now = answer.data;
      if (now) setRunning((was) => (same(was, now) ? was : now));
    });
  }, [projectId]);

  const told = inTauri();
  whileWatched(look, EVERY_MS, projectId, !told);
  useEffect(() => {
    if (!told || !projectId) return;
    void look();
    const stop = onCarried<string>(
      "panes:changed",
      (changed) => changed === projectId && void look(),
    );
    const back = (): void => void (!document.hidden && look());
    document.addEventListener("visibilitychange", back);
    return () => {
      stop();
      document.removeEventListener("visibilitychange", back);
    };
  }, [look, projectId, told]);

  return running;
}
