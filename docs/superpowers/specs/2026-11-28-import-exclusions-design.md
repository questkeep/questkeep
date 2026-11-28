# Import exclusions

## Intent

A store game the user removes from the library comes back on the next sync today: `Library::remove_game`
deletes the row, and `upsert_imported` inserts every scanned game it cannot match by
`(source, source_game_id)`. Removing should be able to mean "gone for good", with a way back.

## Goals

- The Remove confirmation (single game, game page, multi-select) gets a checkbox
  "Don't import again when syncing", **ticked by default** (user's choice).
- Ticked: each store copy of the removed entries is remembered and skipped by every later sync.
- Settings lists the excluded games and can bring each one back.
- Removing a combined entry removes all of its copies (today only the primary row is deleted and
  the other copies fall out as separate games through `merged_into … ON DELETE SET NULL`).

## Non-goals

- Excluding by pattern or excluding games before they were ever imported.
- Keeping a removed game's data (running time, edits) for a later return: Hide covers that.

## Approaches

1. **Exclusion table (picked).** Migration 20 adds
   `import_exclusions (source TEXT, source_game_id TEXT, name TEXT, excluded_at INTEGER,
   PRIMARY KEY (source, source_game_id))`. Remove still deletes; `upsert_imported` skips listed ids.
   Game queries stay untouched; backups include it as part of the database.
2. Tombstone flag on `games` (`removed_at`): would restore data on return, but every query on
   `games` needs a filter and one miss shows removed games again. Rejected.
3. A list in `settings.json`: no transaction with the delete, syncs would read settings. Rejected.

## Design

- **Core** (`db/exclusions.rs` + `library.rs`):
  - `remove_games(ids, exclude: bool)` (and `remove_game` through it): in one transaction, collect the
    entries' copies (`merged_into = id`), add an exclusion for each copy with a `source_game_id`
    when `exclude`, delete copies and entries. Activity log: one Removed group as today; the entry
    notes that the games were excluded.
  - `upsert_imported` loads the provider's exclusions once and drops matching scanned games before
    inserting or updating. A row that still exists for an excluded id (should not happen) is left
    alone.
  - `list_exclusions() -> Vec<ImportExclusion>` (ts-rs exported: source, sourceGameId, name,
    excludedAt) and `remove_exclusion(source, source_game_id)`, logged in the activity log.
- **Commands**: `remove_game(id, exclude)`, `remove_games(ids, exclude)`, `list_import_exclusions`,
  `remove_import_exclusion`; thin wrappers.
- **UI**:
  - `ConfirmDialog` gets an optional checkbox (label, default, value passed to `onConfirm`). Shown
    only when at least one affected entry has a store copy (manual games have nothing to exclude).
  - Settings > Stores and accounts: an "Excluded games" card (shown when the list is not empty):
    store icon, name, date, an "Import again" button per row. "Import again" removes the
    exclusion and starts a sync of that store so the game reappears.
  - Texts in en, de, fr, it, es, pt; keyboard reachable; settings search entry.
- **Docs**: wiki user pages on removing games and stores; roadmap "Done in the next release";
  changeset (minor).

## My calls (check these)

- Checkbox only for entries with store copies; label wording above.
- "Import again" syncs that store right away rather than waiting for the next sync.
- Exclusions of a store that is later removed or uninstalled (plugin) stay; they are harmless and
  apply again if the store returns.

## Left to the builder

- Exact activity-log wording and whether "Import again" is a System or Library entry, following
  `2026-11-27-activity-log-design.md`.
