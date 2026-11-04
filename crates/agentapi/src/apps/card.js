// devpit_card as a page.
let project
let asked = {}
devpit.start({
  input: (args) => {
    asked = args || {}
    project = asked.project
  },
  // The agent was answered a summary; the page reads the card whole.
  result: async () => {
    const detail = await devpit.call('devpit_card', { ...asked, full: true }).catch((err) => err.message)
    if (detail && detail.card) cardView.render(project, detail, null)
    else devpit.show(devpit.el('p', { class: 'err', text: String(detail) }))
  },
})
