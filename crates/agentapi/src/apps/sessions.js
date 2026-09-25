// devpit_sessions as a page: who is busy, who is idle, who waits on the
// person — read only. A waiting question is answered in devpit, by the
// person, never from here.
const { el } = devpit
const draw = (sessions) => {
  const waiting = sessions.filter((one) => one.waiting)
  devpit.show(
    el('div', { class: 'row' },
      el('h1', { class: 'grow', text: 'Sessions' }),
      el('span', { class: 'muted', text: `${sessions.length} running${waiting.length ? ` · ${waiting.length} waiting on you` : ''}` }),
      el('button', { text: 'Refresh', onclick: async () => draw(await devpit.call('devpit_sessions', {})) })),
    sessions.length === 0 ? el('p', { class: 'muted', text: 'No session is running.' }) : null,
    el('ul', { class: 'list' }, sessions.map((one) =>
      el('li', { class: 'item' },
        el('span', { class: 'dot', 'data-s': one.waiting ? 'waiting' : one.status }),
        el('div', { class: 'grow' },
          el('div', { class: 'row' }, el('strong', { text: one.name }), el('span', { class: 'muted', text: [one.projectName || 'outside devpit', one.cardId ? 'card' : null].filter(Boolean).join(' · ') })),
          one.waiting ? el('div', {}, el('div', { text: one.waiting.question }), el('div', { class: 'muted', text: `${one.waiting.options.map((option, at) => `${at + 1}. ${option.label}`).join('   ')} — answer it in devpit's Sessions panel` })) : null)))))
}
devpit.start({
  result: (result) => {
    const sessions = devpit.parse(result)
    if (Array.isArray(sessions)) draw(sessions)
    else devpit.show(el('p', { class: 'err', text: String(sessions) }))
  },
})
