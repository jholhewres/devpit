# The community

devpit's community is a Discord server: <https://discord.gg/yW8VUyY63b>. The
app links to it from the account menu and from the foot of Settings, and opens
it in the system's browser.

Everything the server says is said by the devpit bot or by a webhook, never by
a person's account.

## `#releases`

Posted by the release workflow, after the manifests: the version, up to five
highlights from its `CHANGELOG.md` section and the link to the release. The
webhook's URL is the repository secret `DISCORD_RELEASES_WEBHOOK`; without it
the step says so and posts nothing, and a failed post never fails a release.

To read what a version would post, run it locally:

```sh
cargo xtask release-announcement 0.1.38 https://github.com/jholhewres/devpit/releases/tag/v0.1.38
```

or run the *Announce* workflow by hand (Actions → Announce), which prints it,
and posts it only when *Post it* is ticked. It publishes no release.

## `#announcements`

For posts written by hand. It has a webhook of its own, called *devpit*.

**A webhook URL is a password.** Anyone holding it can post as devpit. It is
never committed, pasted into a card, an issue, a log or a chat. Copy it when
you need it from Discord (Server Settings → Integrations → Webhooks →
*devpit* → Copy Webhook URL) and keep it in your password manager.

1. Write the post as JSON in a file outside the repository:

   ```json
   {
     "allowed_mentions": { "parse": [] },
     "embeds": [{
       "title": "Title",
       "description": "What changed, in a few lines.",
       "color": 14842203
     }]
   }
   ```

   `allowed_mentions` stops `@everyone` from pinging anyone by accident.
   Remove it only for a post that should ping.

2. Post it, typing the URL at a prompt so it stays out of your shell history:

   ```sh
   read -rs DISCORD_WEBHOOK && curl -fsS -H 'Content-Type: application/json' \
     --data @post.json "$DISCORD_WEBHOOK" && unset DISCORD_WEBHOOK
   ```

If a URL leaks, delete that webhook in Discord and make a new one: the old URL
stops working at once. For `#releases`, update the repository secret with
`gh secret set DISCORD_RELEASES_WEBHOOK`.
