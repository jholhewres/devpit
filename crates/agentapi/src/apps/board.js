// devpit_board as a page: the lanes and their cards; a card opens here.
let project
const { el, call } = devpit
const draw = (columns) =>
  devpit.show(
    el('div', { class: 'row' }, el('h1', { class: 'grow', text: 'Board' }), el('span', { class: 'muted', text: `${columns.reduce((all, one) => all + one.cards.length, 0)} cards` })),
    el('div', { class: 'lanes' }, columns.map((column) =>
      el('div', { class: 'lane' },
        el('div', { class: 'row' }, el('h2', { class: 'grow', text: column.name }), el('span', { class: 'muted', text: String(column.cards.length) })),
        column.cards.map((card) =>
          el('div', { class: 'card' },
            el('button', { class: 'link', text: card.title, onclick: async () => cardView.render(project, await call('devpit_card', { project, cardId: card.id }), () => draw(columns)) }),
            el('div', { class: 'row muted' }, card.comments ? el('span', { text: `${card.comments} comments` }) : null, card.lastRun ? el('span', { class: 'chip', text: card.lastRun }) : null)))))))
devpit.start({
  input: (args) => (project = args && args.project),
  result: (result) => {
    const columns = devpit.parse(result)
    if (Array.isArray(columns)) draw(columns)
    else devpit.show(el('p', { class: 'err', text: String(columns) }))
  },
})
