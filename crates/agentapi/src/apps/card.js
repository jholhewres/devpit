// devpit_card as a page.
let project
devpit.start({
  input: (args) => (project = args && args.project),
  result: (result) => {
    const detail = devpit.parse(result)
    if (detail && detail.card) cardView.render(project, detail, null)
    else devpit.show(devpit.el('p', { class: 'err', text: String(detail) }))
  },
})
