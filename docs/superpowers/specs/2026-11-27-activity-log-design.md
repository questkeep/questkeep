# Activity log — design

Date: 2026-11-27. Mockups: `docs/mockups/activity-log.html` (game page + drawer, ways in, Activity
page Library/System, retention settings).

## Why

People want to see what happens to their library and why, above all when plugins and store syncs
change things behind their back ("why did my cover change?", "what did plugin X do?"). Mainly for
users; secondarily for the developer debugging plugins.

## Goals

- Every change to library data becomes an **entry**: *who* (you, a store sync, a plugin, Questkeep),
  a one-line **summary of why** ("You refreshed metadata from IGDB"), and expandable **details of
  what** (field, before → after).
- Changes made together (one sync, one bulk edit, one plugin run) form a **batch**: one line in the
  global log that expands to the games it touched; each game's log shows its part.
- A per-game view and a global view; the global view has a **Library** part and a **System** part
  (what the app and its plugins did behind the scenes), both with filters.
- **Play sessions appear in the timeline** (game drawer and Activity page) as their own kind of
  entry, filterable. They are read from the existing `play_sessions` table, not copied.
- Retention configurable, based on size on disk.
- Included in backups.

## Non-goals (for now)

- Undo / revert from the log.
- Compression (values are plain text; revisit if the size limit proves tight).
- Ctrl+K entries (the command bar lists games only; commands come later).
- A sidebar entry.
- Tamper-proof attribution (see "Attribution of extension plugins").

## Anti-goals

- A log so noisy (heartbeats, internal markers, re-resolves that change nothing) that real changes
  drown. Entries with no visible change are not written.
- Slowing down syncs or the UI noticeably.
- Growing `library.sqlite` or making library backups much larger.

## Approaches considered

1. **Recorded at the write sites** (core passes who/why; a generic before/after diff of the game).
   Exact summaries; risk: a forgotten write site is silently missing.
2. **SQLite triggers** on the library tables. Nothing missed, but no who/why, only raw columns (JSON
   blobs, internal flags) that need translating back, and every write (big syncs) gets slower.
3. **Option 1 in its own database file `activity.sqlite`.** Library DB stays small, size-based
   retention is trivial (file size), it can be backed up on its own terms.

**Chosen: 3** (the user picked it). The "forgotten write site" risk is handled by routing writes
through one recorder and a debug-build check (see "Coverage").

## Design

### Storage: `<data>/activity.sqlite`

Own `Database`-like wrapper (`crates/questkeep-core/src/activity/`), own append-only migrations,
WAL, `auto_vacuum = INCREMENTAL` set at creation. One connection behind a mutex like the library DB.

Tables (names indicative; builder may refine):

- `batches(id, started_at, finished_at, actor_kind, actor_id, reason, args_json, counts_json)` —
  one per sync / bulk action / plugin run / metadata job; `counts_json` = {added, updated, removed,
  ...} filled when it finishes.
- `entries(id, at, batch_id NULL, actor_kind, actor_id, reason, args_json, game_id NULL,
  game_name, kind)` — one per game per action (or a library-level action with `game_id` NULL, e.g.
  "Created category X"). `game_name` is a snapshot so entries of removed games still read well.
  `kind` groups entries for the "What" filter: `added`, `removed`, `metadata`, `edit`, `images`,
  `install`, `combine`, `categories`, `sessions`, `profiles`, `scripts`, `status`, `other`.
- `changes(entry_id, field, before_json, after_json)` — field-level details.
- `system_events(id, at, level, source_kind, source_id, kind, message, details_json)` — the System
  part.

Indexes on `entries(at)`, `entries(game_id, at)`, `entries(batch_id)`, `entries(actor_kind,
actor_id, at)`, `system_events(at)`.

`actor_kind`: `user`, `store` (built-in store sync), `plugin` (a plugin: its store sync, its
extension calls), `app` (Questkeep itself: background fetchers, auto categories, startup work).
`actor_id`: store id / plugin id / null. The UI renders names and icons from providers/plugins and
falls back to the id when the plugin is gone.

`reason` is a stable key (`sync`, `autoSync`, `metadataRefresh`, `metadataFetch`, `edit`,
`combine`, `separate`, `bulkChange`, `remove`, `addManual`, `sessionEdit`, `restoreBackup`, ...)
plus `args_json` (e.g. `{ "source": "igdb" }`); the UI translates. No English sentences stored.

### Recording: the recorder

`Library` owns an `ActivityLog` (handle to the DB). Write paths in the core wrap their mutation:

```rust
let ctx = Change::new(Actor::User, Reason::Edit);
self.activity.record(ctx, &[id], || { /* existing mutation */ })?;
```

- `record` snapshots the affected entries before (`Game` via `get_entry`, plus whatever the kind
  needs: categories, copies, profiles), runs the mutation, snapshots after, diffs, and writes one
  entry per game that actually changed (none if nothing visible changed). The mutation's result is
  returned unchanged; an activity write failure is logged (`tracing::warn`) and never fails the
  user's action.
- Bulk paths (sync `upsert_imported`, `resolve_all`, bulk change) take one bulk snapshot (a single
  query of the affected games) before and after, inside a **batch**. A batch is opened with
  `activity.batch(ctx)` and closed with its counts; entries written inside reference it.
- Snapshots/diffs run on the calling thread; writes to `activity.sqlite` are batched in one
  transaction per `record`/batch so a 2 000-game sync costs one transaction.

**Diff** (`activity/diff.rs`, pure, unit-tested): compares the serialized `Game` field by field with
a field table that says how each field is shown:

- shown with values: name, sorting name (user), description (values kept, UI truncates), release
  date, developers/publishers/genres/tags/features/series/platforms/... as **set diffs** (+ / −),
  scores, completion status, user score, notes, kind, favorite, hidden, installed, install dir,
  play action, categories (by name, set diff), copies (as "Store · qualifier", set diff), links,
  time to beat, install size, detect launches / ignored processes, launch profiles (by name).
- shown as "changed" without values: cover, background, icon (paths are internal; the UI may show a
  thumbnail of the current image only).
- never shown (noise or internal): `extra` internals, metadata checked markers, `fieldSources`,
  `state_version`, search keys, sorting name when derived, `metadataUpdatedAt`; playtime /
  play count / last played (covered by sessions; store-imported playtime changes are summarised on
  the sync batch as a count, not per game).

### Coverage: which write sites record what

| Trigger (actor) | Write sites | Entry |
|---|---|---|
| User (Tauri command) | update_game, set_cover, bulk_change, remove_game(s), add_manual_game, save_game_edits, set_user_image/remove_user_image, combine_games/with_plan, separate_copy, prefer_copy, set_completion_status, categories (create/rename/delete/merge/set on game), launch profiles, scripts, set_metadata_link, refetch_artwork, refresh_metadata(_all), session add/update/delete | per game (bulk → batch) |
| Store sync (store or plugin) | scan_provider → upsert_imported, sync_imported_categories, record_store_playtime | batch per scan (manual / auto / after sign-in / install watch); entries for added, removed (installed→no), renamed, installed/uninstalled, kind, play action, categories |
| App: metadata & cover fetchers | set_source_metadata → resolve, image setters | per game, batched per fetch run; only when the shown value changed |
| App: auto categories, resolve_all, recompute_sorting_names | bulk | batch; only visible changes |
| App: completion `apply_first_start` | status change on first launch | per game |
| Backup restore | after the swap, at next start | one entry "Restored the backup from <date>" |
| Not recorded | heartbeats, state_version, metadata_checked markers, sessions begin/finish (sessions are read live), view/UI state, settings.json, migrations | — |

**Debug-build check:** in debug builds and tests, game-mutating `Database` functions assert that a
recording scope (thread-local, set by `record`/`batch`) is active, except functions explicitly
marked internal (the "Not recorded" row). A forgotten write site fails tests instead of silently
missing from the log.

### Attribution of extension plugins

Extension plugins call the same Tauri commands as the UI (`QuestkeepApi.invoke`). The extension
host adds an IPC header (`Questkeep-Actor: plugin:<id>`) to every invoke it makes on a plugin's
behalf; mutating commands read it through a small `Actor` extractor (custom `CommandArg`) and pass
it to the core, defaulting to `user`. Store plugins are attributed by their store's provider
(provider info knows it is a plugin). This is honest-but-spoofable (a plugin's JavaScript could
call the raw IPC without the header): accepted, the log is informational; noted in DECISIONS.

### Sessions in the timeline

Queries that build the timeline read entries from `activity.sqlite` and sessions from
`library.sqlite` (`play_sessions` joined with game names) and merge by time with keyset paging
(`before: (at, id)`, `limit`). A session renders as "Ran for 1 h 12 min" with the copy/store it
ran under. Sessions are not subject to activity retention (they live as long as the game).
*Changes* to sessions (user edits, deletes, store imports) are normal entries.

### System part

A `tracing` layer (installed by the shell next to the file logger) captures events that opt in
with a target `activity` (e.g. `tracing::info!(target: "activity", source = "plugin:x", kind =
"http", ...)`) and writes them to `system_events`. What emits them:

- plugins: loaded / updated / removed / failed to load; requests per plugin run grouped by host
  (count, bytes, errors; host only, never the URL query or body); plugin log lines at warn and
  above;
- stores: sync started/finished/failed (with counts), sign-in expired;
- app: backups created/restored, update check result, notifications posted (errors/warnings).

Levels: info, warn, error. Filters: source (Questkeep, each store, each plugin), level, kind, time,
text search.

### Retention

Settings (`Settings.activity`): `library_max_mb` (default 50), `system_days` (default 30),
`record_plugin_requests` (default on). A pruning pass runs after batches finish and hourly:

1. delete `system_events` older than `system_days`;
2. while the file is larger than the limit: delete the oldest batches / entries (oldest first, in
   chunks), then `PRAGMA incremental_vacuum`.

Settings › Activity shows the current size (library / system), the limits, and "Clear activity…"
(confirm; library data untouched).

### Backups

`activity.sqlite` is included: snapshot with `VACUUM INTO` like the library DB, added to the
archive; the restore swap includes it. An archive without it (older backups) restores with a fresh
empty activity DB (an old library with today's history would not match). After a restore, the first
start records "Restored the backup from <date>".

### Core API, commands, events

- `ActivityQuery { game_id?, actors?, kinds?, from?, to?, search?, include_sessions, before?,
  limit }` → `ActivityPage { items: Vec<ActivityItem>, next: Option<Cursor> }` where `ActivityItem`
  is an entry (with change count), a batch (with counts, expandable to its entries), or a session.
- `activity_details(entry_id)` → changes; `activity_batch_entries(batch_id, before?, limit)`.
- `system_events(query)`; `activity_stats()` (sizes); `clear_activity(part)`.
- `CoreEvent::ActivityAdded { game_ids }` so open views refresh.
- All types ts-rs exported; commands thin.

### UI

- **Game page:** "Recent activity" card in the side column (last 3 items incl. sessions, "Show all");
  "Activity…" in the page's "…" menu and in the game's right-click menu open a **drawer** from the
  right with the game's full log (Who / What filters, Sessions toggle, expandable entries, link "Open
  in the Activity page" filtered to the game). Esc / close button; focus trapped; keyboard path.
- **Activity page** (a page like Statistics): Library | System switch, filters (search, Who, What,
  When, Sessions toggle), day groups, expandable entries and batches, infinite scroll.
- **Ways in:** Questkeep menu "Activity" (next to Settings) with a shortcut (Ctrl+Shift+A if free,
  otherwise the builder picks a free one); the bell's footer "Show all activity"; a sync result
  toast "Details" (page filtered to that store's latest batch); "Activity" on each plugin in
  Settings › Plugins (filtered to that plugin). Not in Ctrl+K, not in the sidebar.
- **Settings › Activity:** retention and size, record plugin requests, Clear.
- Strings in all six locales; dates relative like the rest of the app; theme variables only.

### Testing

- Core: diff table (each field kind), recorder (no-change → no entry, failure isolation), batch
  counts, each write site row of the coverage table (at least one test per trigger), debug-scope
  assertion catches an unrecorded write, timeline merge with sessions and paging, retention by size
  and age, backup include/restore (with and without `activity.sqlite`), restore entry.
- Shell: actor header extraction.
- UI: vitest for drawer, card, page filters, entry expansion; verified in the hidden app.

## Left to the builder

Exact table/column names, the field-label catalogue, chunk sizes for pruning, how bulk snapshots
are queried, the shortcut if Ctrl+Shift+A is taken, and the System event kinds beyond the list
above.
