/*
 * The board the screens are taken of, made through the app's own commands.
 *
 * Not by writing SQLite: a fixture database is a shape somebody invented, and
 * the first migration that changes makes every screenshot a picture of a state
 * the app can no longer produce. Going through `invoke` means the seed is
 * exactly what a person clicking would have made.
 */

/** Runs one command in the window, the way the window's own client does. */
export async function invoke(window, command, args = {}) {
  const answer = await window.executeAsyncScript(
    function (command, args, done) {
      window.__TAURI_INTERNALS__
        .invoke(command, args)
        .then((data) => done({ ok: true, data }))
        .catch((error) => done({ ok: false, error: String(error?.message ?? error) }))
    },
    command,
    args,
  )
  if (!answer.ok) throw new Error(`${command} refused: ${answer.error}`)
  return answer.data
}

/**
 * A project, a finished first run, a lane that runs something, two cards and
 * a comment. Deterministic: the same calls in the same order, so two runs
 * produce the same board.
 */
export async function seedBoard(window, repo) {
  await invoke(window, 'settings_finish_onboarding')
  const project = await invoke(window, 'project_add', { rootPath: repo })
  // Opened as well as added: the tests share a home, and the window shows
  // whichever project was opened last — another test's, otherwise.
  await invoke(window, 'project_open', { projectId: project.id })
  const board = await invoke(window, 'board_get', { projectId: project.id })

  const step = await invoke(window, 'step_create', {
    projectId: project.id,
    kind: 'command',
    name: 'tests',
    config: JSON.stringify({ command: 'true' }),
    irreversible: false,
  })
  const lane = board.columns[1]
  // The step this call just made, by name — not `steps[0]`. The answer carries
  // every step the project has, and a test file that made one of its own
  // changes which is first: the lane then runs somebody else's step and the
  // board says a word this seed never chose.
  const mine = (step.steps ?? []).find((one) => one.name === 'tests')
  await invoke(window, 'column_set_step', {
    projectId: project.id,
    columnId: lane.id,
    stepId: mine?.id ?? null,
  })

  const first = await invoke(window, 'card_create', {
    projectId: project.id,
    columnId: board.columns[0].id,
    title: 'Fix the parser',
    body: 'It drops the last line of a file that has no newline at the end.',
  })
  await invoke(window, 'card_create', {
    projectId: project.id,
    columnId: board.columns[0].id,
    title: 'Name the socket',
    body: '',
  })
  await invoke(window, 'card_comment', {
    projectId: project.id,
    cardId: first.id,
    body: 'Reproduced on a file of one line.',
  })

  return { project, card: first }
}
