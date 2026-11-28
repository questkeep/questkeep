# Changesets

Every user-visible change adds one small markdown file here. The release workflow turns the pending
files into the next version and the release notes, then deletes them. This README is ignored by the
tooling.

```md
---
bump: minor   # major | minor | patch
---
Add Epic, GOG and Xbox providers.
```

- `bump` is required. The release uses the highest bump of all pending files (major > minor > patch).
  Below 1.0 this is plain semver too: `major` on 0.x goes to 1.0.0.
- The body is the release-note text (one bullet in the notes). Write it for players, not developers.
- The file name is `yyyyMMddHHmmss-<slug>.md` (UTC time, then a few lowercase words) and sets the
  order of the release notes. Create one with the command, which names it; `--slug` picks the
  words (default: the start of the summary):

  ```sh
  pnpm changeset add minor "Add Epic, GOG and Xbox providers." --slug epic-gog-xbox
  pnpm changeset status
  ```

Docs-only, CI-only and refactoring changes need no changeset. See `docs/RELEASING.md`.

## Writing the text

The pending files become one set of release notes, so they must read as one text.

- **Full sentences that end with a period.** Say what players can now do or what changed for
  them, not how it was built. One feature per file; don't pack unrelated features into one bullet.
- **Use the app's current English labels, exactly as the UI shows them**: "Add game menu",
  "Times started", "Don't import again when syncing". Write paths with ` > `:
  `Settings > Stores and accounts`, `Edit > Metadata`. Keys as `Ctrl+K`, `Shift+click`.
- **Mention what users will notice besides the feature itself**: a one-time notification, data
  that is fetched or migrated once, something that moved or was removed and what to do instead.
- Changes only plugin authors see start with "Plugins can …" / "Store plugins can …".
- `patch` for fixes and small adjustments, `minor` for new features and changed behavior
  (including removals, while below 1.0).

## Before adding one: read the pending ones

Run `pnpm changeset status` (or read the files here). Later changes in the same release often
touch what an earlier changeset describes:

- **Your change renames, moves or removes something a pending changeset names** (a label, a
  menu, a settings page): update that changeset's text in the same commit.
- **Your change extends or fixes a feature that is still unreleased** (its changeset is
  pending): edit that changeset instead of adding a new one. Players never saw the broken
  version, so there is nothing to call a fix.
- Don't add a second changeset that repeats one already pending.

## Order

The notes list changesets by file name within each bump section, so oldest timestamp first. When
you edit a pending changeset, decide whether it should move:

- **Keep the name** for wording fixes and small additions: the entry stays where it is.
- **Rename it to the current time** (`git mv`, keep the slug) when your change makes it
  essentially a new or much bigger feature that belongs with the newest entries.

`pnpm changeset status` warns about files without the timestamp; they would sort out of order.
`pnpm check` (and so CI) fails on them.
