# Execution Groups — Design

**Status:** Approved (pending spec review)
**Date:** 2026-04-25

## Summary

Add a second kind of entry — *execution groups* — to keykoff. A group bundles several existing entries (programs or other groups) and, when launched, fires every reachable program. Groups appear in the typeahead overlay alongside programs and are managed from the Commands tab in settings.

The motivating use case: a user with programs `vscode` and `terminal` can create a group `dev-env` that references both. Launching `dev-env` launches both. If `vscode` is later edited (executable path changed, parameters updated, renamed), the change is automatically reflected when `dev-env` is launched — the group references the program, not a snapshot of it.

## Goals

- Bundle multiple programs under a single launchable name.
- Edits to a program automatically propagate to every group that references it (no duplicated configuration).
- Groups can contain other groups (nested), with cycle protection.
- Backwards compatible: existing `config.json` files load unchanged.

## Non-goals

- No per-member overrides (a group cannot launch program A with different parameters than program A's own configuration).
- No sequential launch with delays — fire-and-forget all at once.
- No special visual treatment of groups in the typeahead overlay.
- No rich error UI for partial group launch failures (stderr only, see "Launch error handling" below).

## Decisions (with rationale)

| Question | Decision | Rationale |
|---|---|---|
| Nested groups? | Yes | User wants compositional flexibility (e.g. `full-workspace` = `dev-env` + slack + notion). |
| Reference mechanism? | By name, with cascading rename and delete. | User explicitly chose name-based references but wants edits to propagate, so the app must maintain referential integrity itself when programs are renamed or deleted. |
| Launch order/timing? | All at once, fire-and-forget. | Existing `launcher::launch` already detaches; launching N programs in sequence with no waiting is effectively concurrent. |
| Member-picker UX? | Type-to-add with autocomplete. | Scales to many entries without a long checkbox list. |
| Visual distinction in typeahead? | None | User-created entries; the user knows what they made. Caption is enough context. |

## Data model

Replace the homogeneous `entries: Vec<RunConfig>` with a tagged enum:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Entry {
    Program(RunConfig),
    Group(RunGroup),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunConfig {
    pub name: String,
    #[serde(default)]
    pub caption: String,
    pub executable: String,
    pub parameters: String,
    pub working_directory: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunGroup {
    pub name: String,
    #[serde(default)]
    pub caption: String,
    pub members: Vec<String>,  // names of other entries (programs or groups)
}

pub struct AppConfig {
    pub entries: Vec<Entry>,
    // ... overlay_x, overlay_y, overlay_width, hotkey_*, unchanged
}
```

### Backwards compatibility

Existing `config.json` files have entries without a `"kind"` field — those must load as `Entry::Program`. Implementation: a custom `Deserialize` impl on `Entry` that inspects the JSON object, treats a missing `kind` as `"program"`, then dispatches to the appropriate variant. New entries are always written with explicit `kind`.

Example old (still valid):
```json
{ "name": "mumble", "executable": "C:\\...", "parameters": "", "working_directory": "" }
```

Example new:
```json
{ "kind": "program", "name": "mumble", "executable": "...", "parameters": "", "working_directory": "" }
{ "kind": "group", "name": "dev-env", "caption": "All dev tools", "members": ["vscode", "terminal"] }
```

### Name uniqueness

Names live in a single namespace shared by programs and groups. The save path in both dialogs validates that the name is unique among all other entries.

## Behavior

### Cascading rename

When the user saves a name change for any entry (program or group), the cascade runs **after** the name-uniqueness validation has passed (otherwise a clash could silently rewrite unrelated members). Iterate every `Entry::Group` in `config.entries` and replace each `members` string equal to the old name with the new name. Save the config once after the rewrite.

### Cascading delete

When the user deletes any entry, iterate every `Entry::Group` and remove the deleted name from each `members` list. Empty groups are allowed *as a result of cascade* (they simply launch nothing) — this is intentionally asymmetric with the save-time rule that requires at least one member when creating or editing a group via the dialog. Save the config once after the rewrite.

### Cycle prevention

Two-tier defense:

1. **Edit-time** — In the type-to-add dropdown of the group dialog, candidates are filtered to exclude:
   - the group being edited (self),
   - entries already in `members`,
   - any group that would create a cycle if added (transitively reaches the group being edited).
2. **Launch-time** — A `HashSet<String>` of visited names guards the recursive launch traversal so a cycle introduced via hand-edited config cannot cause infinite recursion.

### Launch semantics

Launching a `Program` calls `launcher::launch` exactly as today.

Launching a `Group` walks the group's `members` depth-first:
- Maintain a visited-set to prevent cycles.
- Maintain a deduped, ordered list of program indices (so the same program reachable via multiple paths only launches once).
- Resolve each member name to an entry index. If a name doesn't resolve (orphan from hand-edit), skip silently.
- After the walk completes, call `launcher::launch` on each collected program in collection order.

### Launch error handling

For a single-program launch (today), a launch failure opens the EditConfig dialog populated with the failing entry and the error message. That pattern doesn't generalize to groups (multiple programs could fail). For v1: group launch errors are written to stderr and the app returns to Idle. The user can debug by launching individual programs from the overlay. This may be revisited if it proves friction-y in practice.

### Validation

| Where | Rule |
|---|---|
| Program save | name non-empty; name unique across all entries; executable non-empty (existing behavior). |
| Group save | name non-empty; name unique across all entries; `members` non-empty (at least one member). |
| Type-to-add suggestions | substring-match against entry names, exclude self, exclude already-added members, exclude cycle-forming candidates. |

## UI

### Commands tab (`config_list.rs`)

A single ordered list of entries (programs and groups intermixed in insertion order). Each row layout reuses the existing right-to-left pattern (Edit/Delete buttons get space first, name + summary fill the rest).

- Right-side summary for programs: `-> {executable}` (truncated; unchanged).
- Right-side summary for groups: `-> N members` or `-> A, B, C` truncated.

The current single "+ New Configuration" button is replaced with two:

```
[ + New Program ]   [ + New Group ]
```

Edit dispatches to the program dialog or group dialog based on the entry kind. Delete is unchanged behaviorally (but now triggers the cascade-delete sweep).

### Program dialog (`config_dialog.rs`)

Unchanged. Continues to handle `AppMode::NewConfig` and `AppMode::EditConfig`. The save path now also performs cascade-rename when the name changed.

### Group dialog (`group_dialog.rs`, new file)

Reuses the same window geometry as the program dialog. Layout:

```
Heading: "New Group" or "Edit Group"
---
Name:        [_________________]
Caption:     [_________________]   (optional)
Members:     [type to add...___]   ← autocomplete dropdown of matches
             - A          [×]
             - B          [×]
             - some-group [×]
[error label, if any]
                                      [ Cancel ]   [ Save ]
```

- The Members textbox shows a dropdown of suggestions while the user types. Up/Down + Enter to pick; Enter on a picked suggestion adds it and clears the textbox.
- Each member row has a × button to remove that member.
- Save validates and, on success, performs cascade-rename and returns to ConfigList (or Idle, per `dialog_return_to_idle`, mirroring program-dialog behavior).
- Escape: if the autocomplete dropdown is open and showing suggestions, Escape closes the dropdown only; otherwise Escape cancels the dialog. (One press to dismiss the dropdown, a second press to leave the dialog.)

### Input overlay (`input_overlay.rs`)

No code changes. Groups already appear because they're in `entries`. The dispatch on Enter / number key / click changes only inside `app.rs::do_launch`.

### App modes

```rust
enum AppMode {
    Idle,
    Input,
    NewConfig,                          // existing — program
    EditConfig { index: usize },        // existing — program
    NewGroup,                           // new
    EditGroup { index: usize },         // new
    ConfigList,
}
```

Window geometry: `NewGroup` / `EditGroup` reuse the same viewport setup as `NewConfig` / `EditConfig` (same dimensions, decorations, position).

## Module/file changes

| File | Change |
|---|---|
| `src/config.rs` | Replace `RunConfig`-as-entry with `Entry` enum + `RunGroup`. Add custom `Deserialize` for `Entry` (default `kind` to `program`). Add helpers: `cascade_rename`, `cascade_delete`, `find_by_name`, `would_cycle`, `flatten_group_to_programs`. |
| `src/app.rs` | Add `AppMode::NewGroup` / `AppMode::EditGroup`. Add group-dialog state: `dialog_members: Vec<String>`, `dialog_member_input: String`. Refactor `do_launch` to dispatch by entry kind. Split `save_dialog_entry` into `save_program_dialog` and `save_group_dialog`; both run cascade-rename when the name changed. |
| `src/ui/config_dialog.rs` | Wire cascade-rename into save (no UI change). |
| `src/ui/group_dialog.rs` | **New.** Group dialog UI (autocomplete dropdown, member list, remove buttons). |
| `src/ui/config_list.rs` | Commands tab: split "+ New Configuration" into two buttons; row summary branches on entry kind; Edit dispatches to the right dialog mode. |
| `src/ui/input_overlay.rs` | No change. |
| `src/ui/mod.rs` | Add `pub mod group_dialog;`. |
| `src/launcher.rs` | No change. |
| `CLAUDE.md` | Update Architecture section (modes, dispatch), Data section (Entry enum example), Project Structure (new file). |
| `CHANGELOG.md` | Add Unreleased entry under "Added". |

## Open questions

None at design time. Remaining decisions are implementation details for the writing-plans phase (e.g. exact autocomplete dropdown widget approach in egui, whether `flatten_group_to_programs` returns indices or owned `RunConfig` clones).
