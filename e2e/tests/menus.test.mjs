/*
 * Plan 15's manual items 1 to 4, driven instead of checked by hand.
 *
 * They were never checked. The checklist says so: skipped to finish. These are
 * what replaces it — each test names the item it stands in for, and what it
 * asserts comes from the board command, not from what the tiles happen to
 * draw.
 */

import { strict as assert } from 'node:assert'
import { after, before, describe, test } from 'node:test'
import { By, until } from 'selenium-webdriver'

import { fill, press, openCardMenu, openLaneMenu, settle, text } from '../lib/drive.mjs'
import { seedRepo } from '../lib/home.mjs'
import { invoke } from '../lib/seed.mjs'
import { insideTheSeededHome, openWindow } from '../lib/session.mjs'

let window
let project

const board = () => invoke(window, 'board_get', { projectId: project.id })
const laneNamed = async (name) => (await board()).columns.find((column) => column.name === name)
const cardTitled = async (title) => (await board()).cards.find((card) => card.title === title)

before(async () => {
  window = await openWindow(process.env.E2E_BINARY)
  await window.wait(until.elementLocated(By.css('#root')), 20000)
  await insideTheSeededHome(window, process.env.E2E_HOME)

  const repo = seedRepo(process.env.E2E_HOME, 'menus')
  await invoke(window, 'settings_finish_onboarding')
  project = await invoke(window, 'project_add', { rootPath: repo })
  await invoke(window, 'project_open', { projectId: project.id })
  const lanes = (await board()).columns
  for (const title of ['Keep me', 'Rename me', 'Archive me', 'Delete me']) {
    await invoke(window, 'card_create', {
      projectId: project.id,
      columnId: lanes[0].id,
      title,
      body: '',
    })
  }
  await invoke(window, 'card_create', {
    projectId: project.id,
    columnId: lanes[5].id,
    title: 'In the last lane',
    body: '',
  })

  await window.navigate().refresh()
  await window.wait(until.elementLocated(By.css('.app')), 20000)
  await settle(1200)
  await press(window, 'Board')
  await settle(800)
})

after(async () => {
  await window?.quit()
})

describe('manual item 1 — the card menu does what it says', () => {
  test('Open opens the card', async () => {
    await openCardMenu(window, 'Keep me')
    await press(window, 'Open')
    await settle(700)
    assert.match(await text(window), /Keep me/)
    await window.findElement(By.css('body')).sendKeys('')
    await settle()
  })

  test('Rename renames it, in the store and not just on the tile', async () => {
    await openCardMenu(window, 'Rename me')
    await press(window, 'Rename')
    await fill(window, 'input[aria-label="Card title"]', 'Renamed')
    assert.ok(await cardTitled('Renamed'), 'the rename did not reach the board')
    assert.equal(await cardTitled('Rename me'), undefined)
  })
})

describe('manual item 3 — archive, undo, restore and delete', () => {
  test('Archive takes the card off the board, and Undo brings it back', async () => {
    await openCardMenu(window, 'Archive me')
    await press(window, 'Archive')
    await press(window, 'Archive')
    await settle(600)
    assert.equal(await cardTitled('Archive me'), undefined, 'the card is still on the board')

    await press(window, 'Undo')
    await settle(600)
    assert.ok(await cardTitled('Archive me'), 'Undo did not bring it back')
  })

  test('an archived card comes back from the Archived list', async () => {
    await openCardMenu(window, 'Archive me')
    await press(window, 'Archive')
    await press(window, 'Archive')
    await settle(600)

    const archived = await window.findElement(By.xpath("//button[starts-with(normalize-space(),'Archived')]"))
    await archived.click()
    await settle(700)
    await press(window, 'Restore')
    await settle(700)
    assert.ok(await cardTitled('Archive me'), 'Restore did not bring it back')
    await press(window, 'Close archived').catch(() => {})
  })

  test('Delete asks first, and deletes only once it is answered', async () => {
    await openCardMenu(window, 'Delete me')
    await press(window, 'Delete…')
    assert.ok(await cardTitled('Delete me'), 'deleted before the question was answered')
    assert.match(await text(window), /Delete this card\?/)

    await press(window, 'Delete')
    await settle(700)
    assert.equal(await cardTitled('Delete me'), undefined, 'the card is still there')
  })
})

describe('manual item 2 — the lane menu', () => {
  test('Rename renames the lane', async () => {
    await openLaneMenu(window, 'refine')
    await press(window, 'Rename')
    await fill(window, '[aria-label="refine name"]', 'triage')
    assert.ok(await laneNamed('triage'), 'the lane was not renamed')
  })

  test('Add card adds one to that lane', async () => {
    await openLaneMenu(window, 'review')
    await press(window, 'Add card')
    await fill(window, 'input[aria-label="New card title"]', 'Added from the menu')
    const added = await cardTitled('Added from the menu')
    const lane = await laneNamed('review')
    assert.ok(added, 'no card was added')
    assert.equal(added.columnId, lane.id)
  })

  test('Move right and Move left change the order', async () => {
    const order = async () => (await board()).columns.map((column) => column.name)
    const before = await order()
    const at = before.indexOf('review')

    await openLaneMenu(window, 'review')
    await press(window, 'Move right')
    await settle(600)
    assert.equal((await order()).indexOf('review'), at + 1)

    await openLaneMenu(window, 'review')
    await press(window, 'Move left')
    await settle(600)
    assert.deepEqual(await order(), before)
  })
})

describe('manual item 4 — a lane deleted with its cards moved', () => {
  test('its cards land in the lane that was picked', async () => {
    const ship = await laneNamed('ship')
    assert.ok((await board()).cards.some((card) => card.columnId === ship.id))

    await openLaneMenu(window, 'ship')
    await press(window, 'Delete…')
    await press(window, 'Delete')
    await settle(700)
    assert.match(await text(window), /Move 1 card out of “ship” first/)

    const target = (await board()).columns.find((column) => column.name === 'check')
    await window.executeScript(function (id) {
      const select = document.querySelector('.ask__box select')
      const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set
      setter.call(select, id)
      select.dispatchEvent(new Event('change', { bubbles: true }))
    }, target.id)
    await press(window, 'Move and delete')
    await settle(900)

    assert.equal(await laneNamed('ship'), undefined, 'the lane is still there')
    const moved = await cardTitled('In the last lane')
    assert.equal(moved?.columnId, target.id, 'the card did not land in the lane that was picked')
  })
})
