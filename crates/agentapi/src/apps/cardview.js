// A card, and what can be done to it from a page: comment, move to a lane
// that runs nothing, and hand it to a new session.
const cardView = (() => {
  const { el, call } = devpit
  const when = (seconds) => (seconds ? new Date(seconds * 1000).toLocaleString() : '')

  const startForm = (project, card, done) => {
    const prompt = el('textarea', { placeholder: 'What the session should do with this card' })
    const own = el('input', { type: 'checkbox', checked: true, id: 'own' })
    const said = el('p', { class: 'err' })
    const go = el('button', {
      class: 'go',
      text: 'Start session',
      onclick: async () => {
        if (!prompt.value.trim()) return (said.textContent = 'Say what it should do.')
        go.disabled = true
        try {
          const started = await call('devpit_start_session', { project, cardId: card.id, prompt: prompt.value.trim(), checkout: own.checked })
          done(started)
        } catch (error) {
          said.textContent = error.message
          go.disabled = false
        }
      },
    })
    return el('div', { class: 'form' },
      el('h2', { text: 'Hand it to a new session' }),
      prompt,
      el('label', { class: 'row muted' }, own, el('span', { text: "In the card's own checkout (a worktree). Unticked: the project's folder." })),
      el('div', { class: 'row' }, go),
      said)
  }

  const render = async (project, detail, back) => {
    const card = detail.card
    const context = await call('devpit_context', project ? { project } : {}).catch(() => null)
    const lanes = ((context && context.columns) || []).filter((one) => !one.runsAStep && one.id !== card.columnId)
    const said = el('p', { class: 'err' })
    const redraw = async () => render(project, await call('devpit_card', { project, cardId: card.id }), back)
    const note = el('textarea', { placeholder: 'A comment' })
    const move = el('select', {}, el('option', { value: '', text: 'Move to…' }), lanes.map((one) => el('option', { value: one.id, text: one.name })))
    move.addEventListener('change', async () => {
      if (!move.value) return
      try {
        await call('devpit_move_card', { project, cardId: card.id, columnId: move.value })
        await redraw()
      } catch (error) {
        said.textContent = error.message
      }
    })
    let starting = null
    const startSlot = el('div')
    devpit.show(
      back && el('div', {}, el('button', { class: 'link muted', text: '← Board', onclick: back })),
      el('div', { class: 'row' }, el('h1', { class: 'grow', text: card.title }), el('span', { class: 'chip', text: detail.columnName })),
      card.body ? el('div', { class: 'body', text: card.body }) : el('p', { class: 'muted', text: 'No description.' }),
      el('div', { class: 'row' },
        lanes.length ? move : null,
        el('button', { text: 'Start session…', onclick: () => {
          if (starting) return
          starting = startForm(project, card, (started) => startSlot.replaceChildren(el('p', { class: 'muted', text: `Started ${started.name} in ${started.cwd}.` })))
          startSlot.replaceChildren(starting)
        } })),
      startSlot,
      said,
      el('h2', { text: `Comments · ${detail.comments.length}` }),
      el('ul', { class: 'list' }, detail.comments.map((one) =>
        el('li', { class: 'item' }, el('div', { class: 'grow' }, el('div', { class: 'muted', text: `${one.author} · ${when(one.createdAt)}` }), el('div', { text: one.body }))))),
      el('div', { class: 'row' }, note, el('button', { text: 'Comment', onclick: async () => {
        if (!note.value.trim()) return
        try {
          await call('devpit_comment', { project, cardId: card.id, body: note.value.trim() })
          await redraw()
        } catch (error) {
          said.textContent = error.message
        }
      } })),
      detail.runs.length ? el('h2', { text: `Runs · ${detail.runs.length}` }) : null,
      detail.runs.length ? el('ul', { class: 'list' }, detail.runs.slice(0, 5).map((run) => el('li', { class: 'item' }, el('span', { class: 'chip', text: run.state }), el('span', { class: 'muted', text: when(run.startedAt) })))) : null,
    )
  }
  return { render }
})()
