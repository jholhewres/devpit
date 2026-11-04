import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useRunning } from "./useRunning";

const asked = vi.fn((_projectId: string) =>
  Promise.resolve({ data: [], error: null }),
);
vi.mock("./live", () => ({
  ask: (call: () => unknown) => call(),
  commands: { sessionRunning: (projectId: string) => asked(projectId) },
}));

let told: ((projectId: string) => void) | null = null;
vi.mock("./window", () => ({
  inTauri: () => true,
  onCarried: (name: string, then: (projectId: string) => void) => {
    if (name === "panes:changed") told = then;
    return () => (told = null);
  },
}));

describe("a project's panes", () => {
  beforeEach(() => {
    asked.mockClear();
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());

  /* Every ask forked tmux and ps; the app now says when something changed. */
  it("are asked once, then again only when the app says they changed", async () => {
    const { unmount } = renderHook(() => useRunning("p1"));
    await act(async () => void vi.advanceTimersByTime(10_000));
    expect(asked).toHaveBeenCalledTimes(1);

    await act(async () => told?.("p2"));
    expect(asked).toHaveBeenCalledTimes(1);
    await act(async () => told?.("p1"));
    expect(asked).toHaveBeenCalledTimes(2);
    unmount();
  });
});
