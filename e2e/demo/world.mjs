/*
 * The made-up company the launch screenshots are taken of.
 *
 * Every name here is invented and neutral: no person, client, path or address
 * that exists. The seed and the demo agent both read it, so the card an
 * orchestrator "starts" is a card the board really has.
 */

export const PERSON = { name: 'Sam Rivera', email: 'sam@example.com' }

/** Lanes by their default names: inbox, refine, review, doing, check, ship. */
export const PROJECTS = [
  {
    name: 'acme-api',
    files: {
      'README.md': '# acme-api\n\nOrders, billing and search for the Acme store.\n',
      'src/orders/list.ts':
        "import { db } from '../db'\n\nexport async function listOrders(customerId: string) {\n  return db.orders.findMany({ where: { customerId } })\n}\n",
      'src/checkout/cart.test.ts':
        "import { test, expect } from 'vitest'\nimport { checkout } from './cart'\n\ntest('checkout charges the cart total', async () => {\n  const receipt = await checkout(cartWith(3))\n  expect(receipt.total).toBe(4497)\n})\n",
      'src/webhooks/deliver.ts':
        "export async function deliver(hook: Webhook, body: string) {\n  const response = await fetch(hook.url, { method: 'POST', body })\n  if (!response.ok) throw new Error(`delivery failed: ${response.status}`)\n}\n",
    },
    cards: [
      { lane: 'inbox', title: 'Rate-limit the public search endpoint', body: 'Bots hit /search hard at night. 60 requests a minute per key.' },
      { lane: 'inbox', title: 'Add OpenTelemetry traces to the job runner', body: '' },
      { lane: 'refine', title: 'Split billing webhooks into their own worker', body: 'They share a queue with email today, so a slow provider delays receipts.' },
      { lane: 'doing', title: 'Fix the flaky checkout integration test', body: 'Fails about one run in ten on CI, never locally.', session: 'checkout' },
      { lane: 'doing', title: 'Migrate the sessions table to UUID v7', body: 'Sortable ids, no more sequence contention.', session: 'uuid' },
      { lane: 'doing', title: 'Retry failed webhook deliveries with backoff', body: 'Today a 502 from a customer endpoint drops the event.', session: 'webhooks' },
      { lane: 'check', title: 'Paginate the /orders endpoint', body: 'Cursor-based, 50 per page.', diff: 'paginate', review: true },
      { lane: 'check', title: 'Upgrade to Postgres 17', body: '', review: true },
      { lane: 'ship', title: 'Cache product thumbnails at the edge', body: '', review: true },
      { lane: 'ship', title: 'Remove the legacy v1 auth routes', body: '', review: true },
      { lane: 'ship', title: 'Add request ids to every log line', body: '' },
    ],
    remind: { title: 'Review the release notes before the Friday deploy', lane: 'inbox' },
  },
  {
    name: 'storefront',
    files: { 'README.md': '# storefront\n\nThe Acme web shop.\n' },
    cards: [
      { lane: 'inbox', title: 'Dark mode for the product page', body: '' },
      { lane: 'doing', title: 'Lazy-load reviews below the fold', body: '', session: 'reviews' },
      { lane: 'check', title: 'Fix the layout shift on the hero banner', body: '', review: true },
      { lane: 'ship', title: 'Show delivery estimates at checkout', body: '' },
    ],
  },
  {
    name: 'mobile-app',
    files: { 'README.md': '# mobile-app\n\nThe Acme app for iOS and Android.\n' },
    cards: [
      { lane: 'inbox', title: 'Offline cart for the subway crowd', body: '' },
      { lane: 'refine', title: 'Push notifications for shipped orders', body: '' },
      { lane: 'ship', title: 'Biometric sign-in', body: '', review: true },
    ],
  },
  {
    name: 'docs-site',
    files: { 'README.md': '# docs-site\n\nPublic API reference.\n' },
    cards: [
      { lane: 'inbox', title: 'Document the new pagination cursor', body: '' },
      { lane: 'ship', title: 'Versioned API reference', body: '' },
    ],
  },
]

export const ORCHESTRATOR = 'Release crew'

/** The card the orchestrator hands to a session on camera. */
export const HANDED = { project: 'acme-api', title: 'Fix the flaky checkout integration test' }

/** The card the orchestrator adds on camera. */
export const ADDED = { project: 'storefront', title: 'Cart badge count is stale on Safari' }

/** What the person says to the orchestrator, in order. */
export const CHAT = [
  "What's blocking the release this week?",
  'Start a session on the flaky checkout test and tell me when it needs a decision.',
  'Add a card to storefront: the cart badge count is stale on Safari.',
]
