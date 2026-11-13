# Chat performance

The rules a chat keeps while an answer streams, the tests that hold each one,
and the ruler that measures the whole of it.

A streamed answer is the one thing in the window that changes many times a
second for a long time. Whatever it costs per batch, it costs for the whole of
every long answer, in every chat that is open.

## Rules

**A batch draws the live turn and nothing else.** A conversation's messages
live in a store (`web/src/shell/chatMessages.ts`), not in component state.
Each reader picks what it needs (`useMessagesPick`, `useMessage`) and is drawn
again only when its pick changes: the pane reads the count and the first
prompt, the thread the ids, each row its own message, the composer the turn in
flight. `Composer` and the pane's corner are memoised, and `useChat` hands back
one object while nothing in it changed.
Held by `chatBatchRenders.test.tsx`, which counts what a batch draws through
the real pane and the real chat.

**No timer runs per frame on screen.** The backend sends text at most once
every 50 ms (`apps/desktop/src/chat_pace.rs`). The window shows each batch as
it lands; a new markdown block fades in, and with reduced motion it does not.
Nothing types the text out on a timer. Held by `replyStreams.test.tsx`.

**A frame a window is sent stays under 8 KB.** Tauri evaluates a channel frame
under 8 KB of JSON as one script; a larger one costs the window a fetch. Text
goes in frames of at most 4 KB (`MOST_HELD` in `chat_pace.rs`), a tool's
output is cut to its head and tail and an image is named rather than carried
(`crates/rpc/src/cut.rs`). Held by
`every_frame_a_streaming_answer_sends_a_window_fits_the_budget` in
`chat_relay_tests.rs`, which `make test` runs.

**A frame is cut once.** The relay cuts each frame once for every window
listening and sends the Remote the whole frame. A window was sent the whole
frame, read it back and cut it, every frame.

**Git is off the turn's path.** The checkout's snapshot before a turn is taken
on a thread of its own, so the agent starts without waiting on git; the end
waits for it only if it is not done by then. The turn's end goes to the window
before the snapshot after it, and the changed files arrive as one more part a
moment later. An edit the agent makes before the first snapshot is done counts
as the checkout's, not the turn's. Held by the tests in
`turn_changes_tests.rs`.

**The window's spacers are carried, not summed.** A long thread is drawn as a
window over it (`useWindow.ts`); the spacers above and below are carried from
draw to draw, and a draw that moved nothing reads no row. A row out of view is
never measured, so the height it left with is the one the spacer holds. Held
by the spacer tests in `useWindow.test.ts`.

## The ruler

`e2e/chat-stream.mjs` drives a release build through the WebDriver on a
virtual screen of its own. It opens a chat, has the stub answer with 20 KB of
markdown in 12-character pieces due every 8 ms, and reports, for the time the
answer streams:

| | |
|---|---|
| `frameP50`, `frameP95`, `frameMax`, `framesOver50` | gaps between animation frames |
| `textMovesPerS`, `stillP95`, `stillMax` | how often the text on screen moved, and the longest it stood still |
| `webCpuS`, `appCpuS` | CPU seconds of the web process and of the app's own process |

```sh
./node_modules/.bin/tauri build --debug --no-bundle --config apps/desktop/tauri.conf.json
CHAT_STREAM_BINARY=target/debug/devpit-desktop node e2e/chat-stream.mjs
```

Without `CHAT_STREAM_BINARY` it runs `target/release/devpit-desktop`. A debug
build is enough to compare two changes to the window: the web process runs the
same code either way, and only the app's own CPU is inflated, on both sides.

It is not in `make e2e`: it measures a machine, and a number that moves with
the load on the box would fail a suite for nothing. To compare two builds, run
them interleaved, a few rounds each, on the same machine at the same time, and
compare medians; one run of each says little on a machine doing other work.

### Before and after the light render

`origin/main` at 32a56e30 against this change, both debug builds, six rounds
each, interleaved, the machine's one-minute load between 6 and 11:

| median of 6 | before | after |
|---|---|---|
| web-process CPU over the answer | 14.9 s | 12.3 s |
| app-process CPU | 2.8 s | 2.9 s |
| frame gap p50 | 17 ms | 16.5 ms |
| frame gap p95 | 38.5 ms | 36.5 ms |
| longest frame gap | 67.5 ms | 61.5 ms |
| frames over 50 ms | 4 | 5 |
| times the text moved, per second | 28.4 | 17.5 |
| longest the text stood still, p95 | 41.5 ms | 78.5 ms |

The web process spent less in every pair. Frame gaps moved little, and within
what one round differs from the next. The text moves less often by design:
it moves when a batch lands, about every 50 ms, where a 33 ms timer used to
type it out between batches.

The first runs had frame gaps worse than before (p95 25 to 30 ms): a store's
update draws at once, and the batch was applied inside the animation frame,
so the whole render landed in that frame. Applying it just after the frame is
painted brought them back under the old ones.
