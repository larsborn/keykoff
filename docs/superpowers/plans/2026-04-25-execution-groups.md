# Execution Groups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a second entry kind ("execution group") that bundles existing program/group entries by name and launches every reachable program when invoked. Edits to programs propagate to groups via cascading rename/delete.

**Architecture:** Replace `entries: Vec<RunConfig>` with `entries: Vec<Entry>` where `Entry` is a serde-tagged enum (`Program(RunConfig)` | `Group(RunGroup)`). A custom `Deserialize` impl makes existing JSON files (no `"kind"` field) load as `Program`. Pure helpers for cascade rename, cascade delete, cycle detection, and DFS flatten live in `config.rs` and are unit-tested. UI gains a new `group_dialog.rs` and two new `AppMode` variants; `input_overlay.rs` is unchanged. Group launch walks the tree depth-first with a visited-set guard, deduplicates programs, then calls the existing `launcher::launch` on each.

**Tech Stack:** Rust 2021, eframe/egui 0.30, serde/serde_json 1.

**Spec:** `docs/superpowers/specs/2026-04-25-execution-groups-design.md`

---

## File Structure

| File | Role |
|---|---|
| `src/config.rs` | New types (`Entry`, `RunGroup`); custom `Deserialize` for `Entry`; helpers (`cascade_rename`, `cascade_delete`, `find_by_name`, `would_cycle`, `flatten_group_to_programs`); load/save unchanged in API. **Has unit tests in a `#[cfg(test)] mod tests` block.** |
| `src/app.rs` | Two new `AppMode` variants; dialog state for groups (`dialog_members`, `dialog_member_input`); split `save_dialog_entry` into program + group variants; `do_launch` dispatches by kind. |
| `src/ui/config_dialog.rs` | Unchanged in shape. Save path now invokes cascade-rename when name changes. Operates only on `Entry::Program`. |
| `src/ui/group_dialog.rs` | **New file.** Group dialog UI: name, caption, type-to-add member input with autocomplete dropdown, member list with × remove buttons. |
| `src/ui/config_list.rs` | Commands tab: split "+ New Configuration" into "+ New Program" and "+ New Group"; row summary branches on `Entry` kind; Edit dispatches to the correct dialog mode; Delete invokes cascade-delete. |
| `src/ui/input_overlay.rs` | Unchanged. Filtering by name works for both kinds; launch dispatch happens in `app.rs::do_launch`. |
| `src/ui/mod.rs` | Add `pub mod group_dialog;`. |
| `src/launcher.rs` | Unchanged. |
| `CLAUDE.md` | Update Architecture (modes, dispatch), Data (Entry enum example), Project Structure (new file). |
| `CHANGELOG.md` | Add Unreleased entry under "Added". |

---

## Task 1: Add `Entry` enum + `RunGroup` type with backwards-compatible `Deserialize`

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write the failing tests**

Append to bottom of `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_entry_without_kind_loads_as_program() {
        let json = r#"{"name":"foo","caption":"","executable":"x.exe","parameters":"","working_directory":""}"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        match entry {
            Entry::Program(p) => assert_eq!(p.name, "foo"),
            _ => panic!("expected Program, got {:?}", entry),
        }
    }

    #[test]
    fn entry_with_kind_program_loads_as_program() {
        let json = r#"{"kind":"program","name":"foo","caption":"","executable":"x.exe","parameters":"","working_directory":""}"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        assert!(matches!(entry, Entry::Program(_)));
    }

    #[test]
    fn entry_with_kind_group_loads_as_group() {
        let json = r#"{"kind":"group","name":"g","caption":"","members":["a","b"]}"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        match entry {
            Entry::Group(g) => {
                assert_eq!(g.name, "g");
                assert_eq!(g.members, vec!["a".to_string(), "b".to_string()]);
            }
            _ => panic!("expected Group, got {:?}", entry),
        }
    }

    #[test]
    fn group_round_trip() {
        let g = Entry::Group(RunGroup {
            name: "g".into(),
            caption: "c".into(),
            members: vec!["a".into()],
        });
        let s = serde_json::to_string(&g).unwrap();
        let parsed: Entry = serde_json::from_str(&s).unwrap();
        assert!(matches!(parsed, Entry::Group(_)));
        assert!(s.contains(r#""kind":"group""#));
    }

    #[test]
    fn program_round_trip_writes_kind() {
        let p = Entry::Program(RunConfig {
            name: "p".into(),
            caption: String::new(),
            executable: "p.exe".into(),
            parameters: String::new(),
            working_directory: String::new(),
        });
        let s = serde_json::to_string(&p).unwrap();
        assert!(s.contains(r#""kind":"program""#));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib config::tests`
Expected: compile errors (no `Entry`, no `RunGroup`).

- [ ] **Step 3: Add the types and custom `Deserialize`**

Add to `src/config.rs` (above the existing `AppConfig`, keep `RunConfig` as-is):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunGroup {
    pub name: String,
    #[serde(default)]
    pub caption: String,
    #[serde(default)]
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Entry {
    Program(RunConfig),
    Group(RunGroup),
}

impl<'de> serde::Deserialize<'de> for Entry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut value = serde_json::Value::deserialize(deserializer)?;
        if let Some(obj) = value.as_object_mut() {
            if !obj.contains_key("kind") {
                obj.insert("kind".into(), serde_json::Value::String("program".into()));
            }
        }
        // Re-deserialize through a private helper enum that uses the standard tagged-enum derivation.
        #[derive(serde::Deserialize)]
        #[serde(tag = "kind", rename_all = "lowercase")]
        enum Helper {
            Program(RunConfig),
            Group(RunGroup),
        }
        let h: Helper = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(match h {
            Helper::Program(p) => Entry::Program(p),
            Helper::Group(g) => Entry::Group(g),
        })
    }
}
```

Keep the existing `RunConfig` struct and the existing `AppConfig` (still using `Vec<RunConfig>` for now — Task 6 migrates it).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib config::tests`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "add Entry enum and RunGroup with backward-compat deserializer"
```

---

## Task 2: `find_by_name` helper

**Files:**
- Modify: `src/config.rs` (add helper + tests)

- [ ] **Step 1: Append the failing test inside the existing `mod tests`**

```rust
    fn make_program(name: &str) -> Entry {
        Entry::Program(RunConfig {
            name: name.into(),
            caption: String::new(),
            executable: format!("{}.exe", name),
            parameters: String::new(),
            working_directory: String::new(),
        })
    }

    fn make_group(name: &str, members: &[&str]) -> Entry {
        Entry::Group(RunGroup {
            name: name.into(),
            caption: String::new(),
            members: members.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn entry_name(e: &Entry) -> &str {
        match e {
            Entry::Program(p) => &p.name,
            Entry::Group(g) => &g.name,
        }
    }

    #[test]
    fn find_by_name_returns_index_when_present() {
        let entries = vec![make_program("a"), make_group("g", &["a"]), make_program("b")];
        assert_eq!(find_by_name(&entries, "g"), Some(1));
        assert_eq!(find_by_name(&entries, "b"), Some(2));
    }

    #[test]
    fn find_by_name_returns_none_when_missing() {
        let entries = vec![make_program("a")];
        assert_eq!(find_by_name(&entries, "missing"), None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib config::tests::find_by_name`
Expected: compile errors (function and helpers undefined).

- [ ] **Step 3: Implement `find_by_name` and the public `entry_name` accessor**

Add to `src/config.rs` (outside the `tests` mod):

```rust
pub fn entry_name(entry: &Entry) -> &str {
    match entry {
        Entry::Program(p) => &p.name,
        Entry::Group(g) => &g.name,
    }
}

pub fn find_by_name(entries: &[Entry], name: &str) -> Option<usize> {
    entries.iter().position(|e| entry_name(e) == name)
}
```

Remove the duplicate `entry_name` helper inside `mod tests` (replace its body to call the public one) so the test module uses the public function:

```rust
    fn entry_name(e: &Entry) -> &str { super::entry_name(e) }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib config::tests`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "add entry_name and find_by_name helpers"
```

---

## Task 3: `cascade_rename` helper

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add failing tests**

```rust
    #[test]
    fn cascade_rename_updates_group_members() {
        let mut entries = vec![
            make_program("a"),
            make_program("b"),
            make_group("g", &["a", "b"]),
        ];
        cascade_rename(&mut entries, "a", "a2");
        if let Entry::Group(g) = &entries[2] {
            assert_eq!(g.members, vec!["a2".to_string(), "b".to_string()]);
        } else {
            panic!("expected group at index 2");
        }
    }

    #[test]
    fn cascade_rename_no_op_when_name_not_referenced() {
        let mut entries = vec![make_program("a"), make_group("g", &["b"])];
        cascade_rename(&mut entries, "a", "a2");
        if let Entry::Group(g) = &entries[1] {
            assert_eq!(g.members, vec!["b".to_string()]);
        }
    }

    #[test]
    fn cascade_rename_updates_multiple_groups() {
        let mut entries = vec![
            make_program("a"),
            make_group("g1", &["a"]),
            make_group("g2", &["a", "x"]),
        ];
        cascade_rename(&mut entries, "a", "a2");
        if let Entry::Group(g) = &entries[1] { assert_eq!(g.members, vec!["a2".to_string()]); }
        if let Entry::Group(g) = &entries[2] { assert_eq!(g.members, vec!["a2".to_string(), "x".to_string()]); }
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib config::tests::cascade_rename`
Expected: compile error (function undefined).

- [ ] **Step 3: Implement**

Add to `src/config.rs`:

```rust
pub fn cascade_rename(entries: &mut [Entry], old_name: &str, new_name: &str) {
    if old_name == new_name {
        return;
    }
    for entry in entries.iter_mut() {
        if let Entry::Group(g) = entry {
            for m in g.members.iter_mut() {
                if m == old_name {
                    *m = new_name.to_string();
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test --lib config::tests`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "add cascade_rename helper"
```

---

## Task 4: `cascade_delete` helper

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add failing tests**

```rust
    #[test]
    fn cascade_delete_removes_name_from_group_members() {
        let mut entries = vec![make_group("g", &["a", "b", "c"])];
        cascade_delete(&mut entries, "b");
        if let Entry::Group(g) = &entries[0] {
            assert_eq!(g.members, vec!["a".to_string(), "c".to_string()]);
        }
    }

    #[test]
    fn cascade_delete_can_leave_group_empty() {
        let mut entries = vec![make_group("g", &["a"])];
        cascade_delete(&mut entries, "a");
        if let Entry::Group(g) = &entries[0] {
            assert!(g.members.is_empty());
        }
    }

    #[test]
    fn cascade_delete_no_op_when_name_absent() {
        let mut entries = vec![make_group("g", &["a"])];
        cascade_delete(&mut entries, "z");
        if let Entry::Group(g) = &entries[0] {
            assert_eq!(g.members, vec!["a".to_string()]);
        }
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib config::tests::cascade_delete`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
pub fn cascade_delete(entries: &mut [Entry], name: &str) {
    for entry in entries.iter_mut() {
        if let Entry::Group(g) = entry {
            g.members.retain(|m| m != name);
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib config::tests`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "add cascade_delete helper"
```

---

## Task 5: `would_cycle` helper

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add failing tests**

```rust
    #[test]
    fn would_cycle_self_reference() {
        let entries = vec![make_group("g", &[])];
        // Adding "g" to its own members is a cycle.
        assert!(would_cycle(&entries, "g", "g"));
    }

    #[test]
    fn would_cycle_program_member_is_safe() {
        let entries = vec![make_program("a"), make_group("g", &[])];
        assert!(!would_cycle(&entries, "g", "a"));
    }

    #[test]
    fn would_cycle_indirect() {
        // g1 contains g2; adding g1 to g2's members would form a cycle.
        let entries = vec![make_group("g1", &["g2"]), make_group("g2", &[])];
        assert!(would_cycle(&entries, "g2", "g1"));
    }

    #[test]
    fn would_cycle_unrelated_group_is_safe() {
        let entries = vec![make_group("g1", &[]), make_group("g2", &[])];
        assert!(!would_cycle(&entries, "g1", "g2"));
    }

    #[test]
    fn would_cycle_unknown_candidate_is_safe() {
        let entries = vec![make_group("g", &[])];
        assert!(!would_cycle(&entries, "g", "missing"));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib config::tests::would_cycle`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
/// Returns true iff adding `candidate` to `editing_group_name`'s members would
/// create a cycle (i.e., `candidate` is the group itself, or transitively
/// reaches it via group members).
pub fn would_cycle(entries: &[Entry], editing_group_name: &str, candidate: &str) -> bool {
    if candidate == editing_group_name {
        return true;
    }
    let mut stack = vec![candidate.to_string()];
    let mut visited = std::collections::HashSet::new();
    while let Some(name) = stack.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        if name == editing_group_name {
            return true;
        }
        if let Some(idx) = find_by_name(entries, &name) {
            if let Entry::Group(g) = &entries[idx] {
                for m in &g.members {
                    stack.push(m.clone());
                }
            }
        }
    }
    false
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib config::tests`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "add would_cycle helper for group membership"
```

---

## Task 6: `flatten_group_to_programs` helper

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add failing tests**

```rust
    #[test]
    fn flatten_returns_indices_of_programs_only() {
        let entries = vec![
            make_program("a"),
            make_program("b"),
            make_group("g", &["a", "b"]),
        ];
        let result = flatten_group_to_programs(&entries, 2);
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn flatten_dedups_programs_reachable_via_multiple_paths() {
        // g1 -> [a, g2]; g2 -> [a]; flatten g1 should yield [a] once.
        let entries = vec![
            make_program("a"),
            make_group("g1", &["a", "g2"]),
            make_group("g2", &["a"]),
        ];
        let result = flatten_group_to_programs(&entries, 1);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn flatten_skips_unknown_member_names() {
        let entries = vec![make_program("a"), make_group("g", &["a", "missing"])];
        let result = flatten_group_to_programs(&entries, 1);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn flatten_handles_cycles_without_infinite_loop() {
        // Hand-crafted cycle: g1 -> g2 -> g1; should return no programs and not loop.
        let entries = vec![
            make_group("g1", &["g2"]),
            make_group("g2", &["g1"]),
        ];
        let result = flatten_group_to_programs(&entries, 0);
        assert_eq!(result, Vec::<usize>::new());
    }

    #[test]
    fn flatten_called_on_program_index_returns_just_that_index() {
        let entries = vec![make_program("a")];
        let result = flatten_group_to_programs(&entries, 0);
        assert_eq!(result, vec![0]);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib config::tests::flatten`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
/// Walks the entry at `start_index` depth-first. If it's a `Program`, returns
/// `[start_index]`. If it's a `Group`, returns the deduped list of program
/// indices reachable transitively through its `members`. Cycles are skipped via
/// a visited-set; unknown member names are skipped silently.
pub fn flatten_group_to_programs(entries: &[Entry], start_index: usize) -> Vec<usize> {
    let mut result = Vec::new();
    let mut seen_indices = std::collections::HashSet::new();
    let mut visited_names = std::collections::HashSet::new();
    let mut stack: Vec<usize> = vec![start_index];

    while let Some(idx) = stack.pop() {
        if idx >= entries.len() {
            continue;
        }
        let name = entry_name(&entries[idx]).to_string();
        if !visited_names.insert(name) {
            continue;
        }
        match &entries[idx] {
            Entry::Program(_) => {
                if seen_indices.insert(idx) {
                    result.push(idx);
                }
            }
            Entry::Group(g) => {
                // Push members in reverse so that the first member is processed first.
                for member in g.members.iter().rev() {
                    if let Some(member_idx) = find_by_name(entries, member) {
                        stack.push(member_idx);
                    }
                }
            }
        }
    }
    result
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib config::tests`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "add flatten_group_to_programs walker with cycle and dedup guards"
```

---

## Task 7: Migrate `AppConfig.entries` to `Vec<Entry>` (and all consumers)

This is the largest task — touches every file that reads `entries`. Programs continue to work end-to-end after this; groups appear in the list but cannot yet be created/edited (that's Tasks 8–11).

**Files:**
- Modify: `src/config.rs`, `src/app.rs`, `src/ui/config_list.rs`, `src/ui/input_overlay.rs`

- [ ] **Step 1: Update `AppConfig.entries` type**

In `src/config.rs`, change:

```rust
pub entries: Vec<RunConfig>,
```

to:

```rust
pub entries: Vec<Entry>,
```

- [ ] **Step 2: Update `app.rs` consumers**

In `src/app.rs`:

- `update_filtered_results`: replace the iterator body so it filters using `entry_name`. The filtered list contains indices into `config.entries` (no kind filtering — both programs and groups are matched).

```rust
pub fn update_filtered_results(&mut self) {
    let query = self.search_text.to_lowercase();
    if query.is_empty() {
        self.filtered_indices.clear();
    } else {
        self.filtered_indices = self
            .config
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| crate::config::entry_name(e).to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
    }
    if self.selected_index >= self.filtered_indices.len() {
        self.selected_index = 0;
    }
}
```

- `do_launch`: dispatch on entry kind. For `Program`, behave as before. For `Group`, compute the flattened program list, then launch each. (Cycle protection is already inside `flatten_group_to_programs`.)

```rust
pub fn do_launch(&mut self, config_index: usize) {
    use crate::config::{flatten_group_to_programs, Entry};
    match self.config.entries.get(config_index) {
        Some(Entry::Program(_)) => {
            // Clone the program so we can subsequently call &mut self methods
            // (e.g. populate_program_dialog_from_index) without a borrow conflict.
            let program = if let Some(Entry::Program(p)) = self.config.entries.get(config_index) {
                p.clone()
            } else {
                return;
            };
            if let Err(e) = crate::launcher::launch(&program) {
                self.populate_program_dialog_from_index(config_index);
                self.dialog_error = Some(e);
                self.dialog_return_to_idle = true;
                self.set_mode(AppMode::EditConfig { index: config_index });
            } else {
                self.set_mode(AppMode::Idle);
            }
        }
        Some(Entry::Group(_)) => {
            let program_indices = flatten_group_to_programs(&self.config.entries, config_index);
            for idx in program_indices {
                if let Some(Entry::Program(p)) = self.config.entries.get(idx) {
                    if let Err(e) = crate::launcher::launch(p) {
                        eprintln!("group launch: '{}' failed: {}", p.name, e);
                    }
                }
            }
            self.set_mode(AppMode::Idle);
        }
        None => {
            self.set_mode(AppMode::Idle);
        }
    }
}
```

Rename `populate_dialog_from_entry` to `populate_program_dialog_from_index` to make the intent explicit (this function only handles the program case). Inside, change the body to pattern-match `Entry::Program`:

```rust
pub fn populate_program_dialog_from_index(&mut self, index: usize) {
    if let Some(crate::config::Entry::Program(p)) = self.config.entries.get(index) {
        self.dialog_name = p.name.clone();
        self.dialog_caption = p.caption.clone();
        self.dialog_executable = p.executable.clone();
        self.dialog_parameters = p.parameters.clone();
        self.dialog_working_directory = p.working_directory.clone();
        self.dialog_error = None;
    }
}
```

Update `save_dialog_entry` to wrap a saved program in `Entry::Program`:

```rust
pub fn save_dialog_entry(&mut self) -> bool {
    if self.dialog_name.trim().is_empty() {
        self.dialog_error = Some("Name is required.".into());
        return false;
    }
    if self.dialog_executable.trim().is_empty() {
        self.dialog_error = Some("Executable path is required.".into());
        return false;
    }

    let program = RunConfig {
        name: self.dialog_name.trim().to_string(),
        caption: self.dialog_caption.trim().to_string(),
        executable: self.dialog_executable.trim().trim_matches('"').to_string(),
        parameters: self.dialog_parameters.trim().to_string(),
        working_directory: self.dialog_working_directory.trim().to_string(),
    };

    match &self.mode {
        AppMode::NewConfig => self.config.entries.push(crate::config::Entry::Program(program)),
        AppMode::EditConfig { index } => self.config.entries[*index] = crate::config::Entry::Program(program),
        _ => {}
    }

    if let Err(e) = config::save_config(&self.config) {
        self.dialog_error = Some(format!("Failed to save: {}", e));
        return false;
    }
    true
}
```

(Cascade-rename will be wired in Task 11 — leave the save path as-is for now.)

- [ ] **Step 3: Update `ui/config_list.rs`**

Replace the row-rendering loop's body so it pulls the name/summary via pattern-matching on `Entry`. The right-side summary differs by kind:

```rust
for (i, entry) in app.config.entries.iter().enumerate() {
    let (name, summary): (&str, String) = match entry {
        crate::config::Entry::Program(p) => (p.name.as_str(), format!("-> {}", p.executable)),
        crate::config::Entry::Group(g) => {
            let summary = if g.members.is_empty() {
                "-> (empty)".to_string()
            } else {
                format!("-> {} member{}", g.members.len(), if g.members.len() == 1 { "" } else { "s" })
            };
            (g.name.as_str(), summary)
        }
    };

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Delete").clicked() {
                action = Some(ListAction::Delete(i));
            }
            if ui.button("Edit").clicked() {
                action = Some(ListAction::Edit(i));
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_sized(
                    [max_name_width, ui.spacing().interact_size.y],
                    egui::Label::new(egui::RichText::new(name).strong()),
                );
                ui.add(egui::Label::new(summary).truncate());
            });
        });
    });
    ui.separator();
}
```

Update the `max_name_width` calculation accordingly (use `crate::config::entry_name(e)` instead of `e.name`).

For now, in the action handler, route Edit to `AppMode::EditConfig` only when the entry is a `Program`; ignore Edit on Group (it'll be wired in Task 11):

```rust
Some(ListAction::Edit(i)) if i == usize::MAX => {
    app.clear_dialog_fields();
    app.set_mode(AppMode::NewConfig);
}
Some(ListAction::Edit(i)) => match app.config.entries.get(i) {
    Some(crate::config::Entry::Program(_)) => {
        app.populate_program_dialog_from_index(i);
        app.set_mode(AppMode::EditConfig { index: i });
    }
    Some(crate::config::Entry::Group(_)) => {
        // Wired in Task 11.
    }
    None => {}
},
```

- [ ] **Step 4: Update `ui/input_overlay.rs`**

The overlay reads `app.config.entries[config_idx]` to render text. Update to use `entry_name` and the kind-appropriate caption:

```rust
let (name, caption): (&str, &str) = match &app.config.entries[config_idx] {
    crate::config::Entry::Program(p) => (p.name.as_str(), p.caption.as_str()),
    crate::config::Entry::Group(g) => (g.name.as_str(), g.caption.as_str()),
};
let text = if caption.is_empty() {
    format!("{}  {}", display_idx + 1, name)
} else {
    format!("{}  {} - {}", display_idx + 1, name, caption)
};
```

Apply the same change in the width-measurement block above (same pattern). Also update the "no match → open NewConfig with prefilled name" branch — that still creates a program (unchanged behavior; you can only stumble into NewGroup via the Commands tab).

Update the right-click → open-edit branch (`open_edit`): only act if the entry is a program; ignore right-click on a group for v1 (rationale: editing a group from the overlay is niche and adds UI complexity. Add a one-line `// Group right-click edit not supported in overlay; use Commands tab.` comment to make this explicit).

```rust
fn open_edit(app: &mut KeykoffApp, config_idx: usize) {
    if matches!(app.config.entries.get(config_idx), Some(crate::config::Entry::Program(_))) {
        app.populate_program_dialog_from_index(config_idx);
        app.dialog_return_to_idle = true;
        app.set_mode(AppMode::EditConfig { index: config_idx });
    }
}
```

- [ ] **Step 5: Build and run smoke test**

Run: `cargo build`
Expected: clean build (warnings about unused functions are acceptable for now).

Manual smoke test:
- Run `cargo run`, verify the app starts, tray icon appears, hotkey opens overlay, an existing program in `config.json` still launches.
- If you have no existing `config.json`, create one program via the dialog and verify it appears in the typeahead and launches.

- [ ] **Step 6: Verify the unit-test suite still passes**

Run: `cargo test --lib`
Expected: all `config::tests` passing.

- [ ] **Step 7: Commit**

```bash
git add src/config.rs src/app.rs src/ui/config_list.rs src/ui/input_overlay.rs
git commit -m "migrate AppConfig.entries to Vec<Entry>; programs work end-to-end"
```

---

## Task 8: Add `NewGroup` / `EditGroup` modes and group-dialog state

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add the mode variants**

Extend the `AppMode` enum:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Idle,
    Input,
    NewConfig,
    EditConfig { index: usize },
    NewGroup,
    EditGroup { index: usize },
    ConfigList,
}
```

- [ ] **Step 2: Add dialog state fields on `KeykoffApp`**

In the struct definition, add after the existing `// Config dialog state` block:

```rust
    // Group dialog state
    pub dialog_members: Vec<String>,
    pub dialog_member_input: String,
    pub dialog_suggestion_index: usize,
```

In `KeykoffApp::new`, initialize them:

```rust
            dialog_members: Vec::new(),
            dialog_member_input: String::new(),
            dialog_suggestion_index: 0,
```

- [ ] **Step 3: Update `set_mode` to focus the group dialog and clear input**

Extend the match in `set_mode` to handle the new variants identically to `NewConfig`/`EditConfig`:

```rust
            AppMode::NewConfig | AppMode::EditConfig { .. }
            | AppMode::NewGroup | AppMode::EditGroup { .. } => {
                self.needs_focus = true;
            }
```

- [ ] **Step 4: Update `apply_mode_viewport_commands`**

Reuse the same viewport setup. Change the existing `AppMode::NewConfig | AppMode::EditConfig { .. } =>` arm to also include the new variants:

```rust
            AppMode::NewConfig | AppMode::EditConfig { .. }
            | AppMode::NewGroup | AppMode::EditGroup { .. } => {
                // existing body unchanged
            }
```

- [ ] **Step 5: Add helpers for the group dialog state**

```rust
pub fn clear_group_dialog_fields(&mut self) {
    self.dialog_name.clear();
    self.dialog_caption.clear();
    self.dialog_members.clear();
    self.dialog_member_input.clear();
    self.dialog_suggestion_index = 0;
    self.dialog_error = None;
}

pub fn populate_group_dialog_from_index(&mut self, index: usize) {
    if let Some(crate::config::Entry::Group(g)) = self.config.entries.get(index) {
        self.dialog_name = g.name.clone();
        self.dialog_caption = g.caption.clone();
        self.dialog_members = g.members.clone();
        self.dialog_member_input.clear();
        self.dialog_suggestion_index = 0;
        self.dialog_error = None;
    }
}
```

- [ ] **Step 6: Update the `update()` match to dispatch the new modes to a placeholder**

Add to the match in `eframe::App::update`:

```rust
            AppMode::NewGroup | AppMode::EditGroup { .. } => {
                ui::group_dialog::show(self, ctx);
            }
```

`ui::group_dialog` doesn't exist yet — that's Task 9. Leaving the call in is fine because it won't compile until then. Build will fail; that's expected at this checkpoint and gets fixed in Task 9.

Alternatively, to keep the build green between tasks, add a temporary stub `pub fn show(_app: &mut KeykoffApp, _ctx: &egui::Context) {}` in a new `src/ui/group_dialog.rs` and add `pub mod group_dialog;` to `src/ui/mod.rs` at this step. Recommended.

Create `src/ui/group_dialog.rs` with stub:

```rust
use eframe::egui;
use crate::app::KeykoffApp;

pub fn show(_app: &mut KeykoffApp, _ctx: &egui::Context) {
    // Implemented in Task 9.
}
```

Add to `src/ui/mod.rs`:

```rust
pub mod group_dialog;
```

- [ ] **Step 7: Build and verify**

Run: `cargo build`
Expected: clean build.

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/ui/group_dialog.rs src/ui/mod.rs
git commit -m "scaffold group dialog modes and state"
```

---

## Task 9: Implement `group_dialog::show`

**Files:**
- Modify: `src/ui/group_dialog.rs`

- [ ] **Step 1: Replace the stub with the full dialog**

The dialog computes its suggestion list once per frame, displays it with the currently-selected suggestion highlighted, and supports Up/Down to move the selection and Enter to add the highlighted one. Escape dismisses the dropdown when it's showing; otherwise cancels the dialog.

```rust
use eframe::egui;

use crate::app::{AppMode, KeykoffApp};
use crate::config::{self, would_cycle, Entry};

fn compute_suggestions(app: &KeykoffApp, editing_name: &str) -> Vec<String> {
    let trimmed = app.dialog_member_input.trim().to_lowercase();
    if trimmed.is_empty() {
        return Vec::new();
    }
    app.config
        .entries
        .iter()
        .filter_map(|e| {
            let name = config::entry_name(e);
            if name == editing_name {
                return None; // exclude self
            }
            if app.dialog_members.iter().any(|m| m == name) {
                return None; // already added
            }
            if !name.to_lowercase().contains(&trimmed) {
                return None;
            }
            if would_cycle(&app.config.entries, editing_name, name) {
                return None;
            }
            Some(name.to_string())
        })
        .take(8)
        .collect()
}

pub fn show(app: &mut KeykoffApp, ctx: &egui::Context) {
    let is_edit = matches!(app.mode, AppMode::EditGroup { .. });
    let title = if is_edit { "Edit Group" } else { "New Group" };

    // Resolve the "name to compare against for cycle detection / self-exclusion."
    // For edits, this is the name the group had when the dialog opened (so renaming
    // the group inside the dialog doesn't change which entry counts as "self").
    // For new groups, the user-typed name is used.
    let editing_name_owned: String = match app.mode {
        AppMode::EditGroup { index } => match app.config.entries.get(index) {
            Some(Entry::Group(g)) => g.name.clone(),
            _ => String::new(),
        },
        _ => app.dialog_name.clone(),
    };

    let suggestions = compute_suggestions(app, &editing_name_owned);
    if app.dialog_suggestion_index >= suggestions.len() {
        app.dialog_suggestion_index = 0;
    }
    let dropdown_open = !suggestions.is_empty();

    // Capture key events before the UI renders the input box so the dialog
    // gets first crack at Up/Down/Enter/Escape (the TextEdit doesn't consume them).
    let up_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
    let down_pressed = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
    let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

    let mut suggestion_clicked: Option<String> = None;
    let mut remove_idx: Option<usize> = None;
    let mut save_clicked = false;
    let mut cancel_clicked = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(title);
        ui.add_space(10.0);

        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let spacing = ui.spacing().item_spacing.x;
        let row_height = ui.spacing().interact_size.y;
        let row_spacing = 8.0;

        let label_width = ["Name:", "Caption:", "Members:"]
            .iter()
            .map(|t| {
                ui.fonts(|f| {
                    f.layout_no_wrap(t.to_string(), font_id.clone(), egui::Color32::WHITE)
                        .size()
                        .x
                })
            })
            .fold(0.0f32, f32::max)
            + spacing;

        let panel_width = ui.available_width();

        // Name
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.add_sized([label_width, row_height], egui::Label::new("Name:"));
            let name_resp = ui.add(
                egui::TextEdit::singleline(&mut app.dialog_name)
                    .desired_width(ui.available_width()),
            );
            if app.needs_focus {
                name_resp.request_focus();
                app.needs_focus = false;
            }
        });
        ui.add_space(row_spacing);

        // Caption
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.add_sized([label_width, row_height], egui::Label::new("Caption:"));
            ui.add(
                egui::TextEdit::singleline(&mut app.dialog_caption)
                    .desired_width(ui.available_width())
                    .hint_text("Optional description"),
            );
        });
        ui.add_space(row_spacing);

        // Members input
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.add_sized([label_width, row_height], egui::Label::new("Members:"));
            ui.add(
                egui::TextEdit::singleline(&mut app.dialog_member_input)
                    .desired_width(ui.available_width())
                    .hint_text("Type to add..."),
            );
        });

        // Suggestions dropdown
        if dropdown_open {
            ui.indent("group_suggestions", |ui| {
                for (i, suggestion) in suggestions.iter().enumerate() {
                    let selected = i == app.dialog_suggestion_index;
                    let label = egui::SelectableLabel::new(selected, suggestion);
                    if ui.add(label).clicked() {
                        suggestion_clicked = Some(suggestion.clone());
                    }
                }
            });
        }
        ui.add_space(row_spacing);

        // Existing members list with × remove buttons
        ui.indent("group_members_list", |ui| {
            for (i, member) in app.dialog_members.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("- {}", member));
                    if ui.button("x").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
        });

        if let Some(ref error) = app.dialog_error {
            ui.add_space(5.0);
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.add_space(15.0);
        ui.horizontal(|ui| {
            ui.set_width(panel_width);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }
                ui.add_space(10.0);
                if ui.button("  Save  ").clicked() {
                    save_clicked = true;
                }
            });
        });
    });

    // Apply UI-loop side effects.
    if let Some(i) = remove_idx {
        app.dialog_members.remove(i);
    }
    if let Some(name) = suggestion_clicked {
        app.dialog_members.push(name);
        app.dialog_member_input.clear();
        app.dialog_suggestion_index = 0;
    }

    // Keyboard navigation for the suggestions dropdown.
    if dropdown_open {
        if down_pressed && app.dialog_suggestion_index + 1 < suggestions.len() {
            app.dialog_suggestion_index += 1;
        }
        if up_pressed && app.dialog_suggestion_index > 0 {
            app.dialog_suggestion_index -= 1;
        }
    }

    // Enter: if the dropdown is open, add the highlighted suggestion; otherwise save.
    if enter_pressed {
        if dropdown_open {
            if let Some(name) = suggestions.get(app.dialog_suggestion_index).cloned() {
                app.dialog_members.push(name);
                app.dialog_member_input.clear();
                app.dialog_suggestion_index = 0;
            }
        } else {
            save_clicked = true;
        }
    }

    // Escape: if the dropdown is showing, dismiss it (clear input); otherwise cancel.
    if escape_pressed {
        if dropdown_open {
            app.dialog_member_input.clear();
            app.dialog_suggestion_index = 0;
        } else {
            cancel_clicked = true;
        }
    }

    if cancel_clicked {
        app.set_mode(AppMode::Idle);
        return;
    }
    if save_clicked {
        let return_to_idle = app.dialog_return_to_idle;
        if app.save_group_dialog() {
            app.dialog_return_to_idle = false;
            app.set_mode(if return_to_idle { AppMode::Idle } else { AppMode::ConfigList });
        }
    }
}
```

- [ ] **Step 2: Add `save_group_dialog` to `KeykoffApp`**

In `src/app.rs`:

```rust
pub fn save_group_dialog(&mut self) -> bool {
    let trimmed_name = self.dialog_name.trim().to_string();
    if trimmed_name.is_empty() {
        self.dialog_error = Some("Name is required.".into());
        return false;
    }
    if self.dialog_members.is_empty() {
        self.dialog_error = Some("At least one member is required.".into());
        return false;
    }

    // Determine the previous name (for cascade-rename when editing).
    let prev_name: Option<String> = match self.mode {
        AppMode::EditGroup { index } => match self.config.entries.get(index) {
            Some(crate::config::Entry::Group(g)) => Some(g.name.clone()),
            _ => None,
        },
        _ => None,
    };

    // Uniqueness: name must not collide with any other entry.
    let collides = self
        .config
        .entries
        .iter()
        .enumerate()
        .any(|(i, e)| {
            let name = crate::config::entry_name(e);
            let is_self = matches!(self.mode, AppMode::EditGroup { index } if index == i);
            !is_self && name == trimmed_name
        });
    if collides {
        self.dialog_error = Some("Name already used by another entry.".into());
        return false;
    }

    let group = crate::config::RunGroup {
        name: trimmed_name.clone(),
        caption: self.dialog_caption.trim().to_string(),
        members: self.dialog_members.clone(),
    };

    match self.mode {
        AppMode::NewGroup => self
            .config
            .entries
            .push(crate::config::Entry::Group(group)),
        AppMode::EditGroup { index } => {
            self.config.entries[index] = crate::config::Entry::Group(group);
        }
        _ => {}
    }

    if let Some(prev) = prev_name {
        if prev != trimmed_name {
            crate::config::cascade_rename(&mut self.config.entries, &prev, &trimmed_name);
        }
    }

    if let Err(e) = config::save_config(&self.config) {
        self.dialog_error = Some(format!("Failed to save: {}", e));
        return false;
    }
    true
}
```

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: clean build (some warnings about unused imports — fix them).

- [ ] **Step 4: Commit**

```bash
git add src/ui/group_dialog.rs src/app.rs
git commit -m "implement group dialog UI and save path"
```

---

## Task 10: Two new-buttons + group edit dispatch in Commands tab

**Files:**
- Modify: `src/ui/config_list.rs`

- [ ] **Step 1: Replace the single "+ New Configuration" button with two**

In `show_commands_tab`, replace:

```rust
if ui.button("+ New Configuration").clicked() {
    app.clear_dialog_fields();
    action = Some(ListAction::Edit(usize::MAX));
}
```

with:

```rust
ui.horizontal(|ui| {
    if ui.button("+ New Program").clicked() {
        app.clear_dialog_fields();
        action = Some(ListAction::NewProgram);
    }
    if ui.button("+ New Group").clicked() {
        app.clear_group_dialog_fields();
        action = Some(ListAction::NewGroup);
    }
});
```

Update the `ListAction` enum:

```rust
enum ListAction {
    NewProgram,
    NewGroup,
    Edit(usize),
    Delete(usize),
}
```

- [ ] **Step 2: Update the action-handling match**

```rust
match action {
    Some(ListAction::NewProgram) => {
        app.clear_dialog_fields();
        app.set_mode(AppMode::NewConfig);
    }
    Some(ListAction::NewGroup) => {
        app.clear_group_dialog_fields();
        app.set_mode(AppMode::NewGroup);
    }
    Some(ListAction::Edit(i)) => match app.config.entries.get(i) {
        Some(crate::config::Entry::Program(_)) => {
            app.populate_program_dialog_from_index(i);
            app.set_mode(AppMode::EditConfig { index: i });
        }
        Some(crate::config::Entry::Group(_)) => {
            app.populate_group_dialog_from_index(i);
            app.set_mode(AppMode::EditGroup { index: i });
        }
        None => {}
    },
    Some(ListAction::Delete(i)) => {
        let deleted_name = app.config.entries.get(i).map(|e| crate::config::entry_name(e).to_string());
        app.config.entries.remove(i);
        if let Some(name) = deleted_name {
            crate::config::cascade_delete(&mut app.config.entries, &name);
        }
        let _ = config::save_config(&app.config);
    }
    None => {}
}
```

- [ ] **Step 3: Remove the now-dead `clear_dialog_fields` helper if unused**

Verify with `cargo build` whether any callers remain. If `clear_dialog_fields` is still used (e.g. by NewProgram path above), keep it.

- [ ] **Step 4: Build and smoke test**

Run: `cargo build`
Expected: clean.

Manual smoke:
1. Run `cargo run`, open settings via tray ("Edit Configurations").
2. Click "+ New Program" → program dialog appears as before, save creates a program.
3. Click "+ New Group" → group dialog appears, type a member name, see suggestions, click to add, Save.
4. Verify group appears in the list with `-> N members` summary.
5. Edit the group: name change → both A/B prove in/out (rename A, then check the group still references A2).
6. Delete a program: confirm it's removed from any groups it was in.

- [ ] **Step 5: Commit**

```bash
git add src/ui/config_list.rs
git commit -m "wire new-group button, edit dispatch, and cascade-delete in commands tab"
```

---

## Task 11: Wire cascade-rename into the program save path

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Update `save_dialog_entry` to capture the previous name and call `cascade_rename` after a successful edit**

```rust
pub fn save_dialog_entry(&mut self) -> bool {
    if self.dialog_name.trim().is_empty() {
        self.dialog_error = Some("Name is required.".into());
        return false;
    }
    if self.dialog_executable.trim().is_empty() {
        self.dialog_error = Some("Executable path is required.".into());
        return false;
    }

    let trimmed_name = self.dialog_name.trim().to_string();

    // Uniqueness check: name must not collide with any other entry.
    let collides = self
        .config
        .entries
        .iter()
        .enumerate()
        .any(|(i, e)| {
            let name = crate::config::entry_name(e);
            let is_self = matches!(self.mode, AppMode::EditConfig { index } if index == i);
            !is_self && name == trimmed_name
        });
    if collides {
        self.dialog_error = Some("Name already used by another entry.".into());
        return false;
    }

    let prev_name: Option<String> = match self.mode {
        AppMode::EditConfig { index } => match self.config.entries.get(index) {
            Some(crate::config::Entry::Program(p)) => Some(p.name.clone()),
            _ => None,
        },
        _ => None,
    };

    let program = RunConfig {
        name: trimmed_name.clone(),
        caption: self.dialog_caption.trim().to_string(),
        executable: self.dialog_executable.trim().trim_matches('"').to_string(),
        parameters: self.dialog_parameters.trim().to_string(),
        working_directory: self.dialog_working_directory.trim().to_string(),
    };

    match self.mode {
        AppMode::NewConfig => self
            .config
            .entries
            .push(crate::config::Entry::Program(program)),
        AppMode::EditConfig { index } => {
            self.config.entries[index] = crate::config::Entry::Program(program);
        }
        _ => {}
    }

    if let Some(prev) = prev_name {
        if prev != trimmed_name {
            crate::config::cascade_rename(&mut self.config.entries, &prev, &trimmed_name);
        }
    }

    if let Err(e) = config::save_config(&self.config) {
        self.dialog_error = Some(format!("Failed to save: {}", e));
        return false;
    }
    true
}
```

- [ ] **Step 2: Build and smoke test**

Run: `cargo build`
Expected: clean.

Manual smoke (end-to-end):
1. Create program A, group G containing A.
2. Edit A, rename to A2, save.
3. Open G — verify member is now "A2".
4. Delete A2 — verify G's member list is empty.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "wire cascade_rename into program save path with uniqueness check"
```

---

## Task 12: Update `CLAUDE.md` and `CHANGELOG.md`

**Files:**
- Modify: `CLAUDE.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Update `CLAUDE.md`**

In the "Architecture" section, add `NewGroup` and `EditGroup` to the mode list with one line each. In the "Project Structure" tree, add `group_dialog.rs`. In the "Data" section, replace the example with one that shows both an entry with `kind: program` and an entry with `kind: group`. In the "Data model" subsection, replace `RunConfig` references with the `Entry` enum and `RunGroup`.

- [ ] **Step 2: Update `CHANGELOG.md`**

Under "Unreleased" → "Added":

```markdown
- Execution groups: bundle multiple programs (or other groups) under a single name; launching a group launches every reachable program. Renaming or deleting a referenced entry automatically updates groups that use it.
```

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md CHANGELOG.md
git commit -m "document execution groups feature"
```

---

## Verification checklist

Before marking the feature done, run through:

- [ ] `cargo test --lib` — all `config::tests` passing.
- [ ] `cargo build` — clean (no warnings introduced beyond pre-existing).
- [ ] `cargo build --release` — builds.
- [ ] Manual smoke:
  - [ ] Existing `config.json` (no `"kind"` field) loads and programs launch unchanged.
  - [ ] Creating a program writes `"kind":"program"` to JSON.
  - [ ] Creating a group writes `"kind":"group"` to JSON.
  - [ ] Group appears in typeahead overlay and launches all members when picked.
  - [ ] Renaming a program updates group members that referenced it.
  - [ ] Deleting a program removes it from groups that referenced it.
  - [ ] Cycle attempt: A group attempting to add itself or a transitively-reaching group is filtered out of the suggestions dropdown.
  - [ ] Hand-edited cycle in JSON does not infinite-loop on launch.
  - [ ] Empty-after-delete group exists in list (with `-> (empty)` summary) and launching it is a silent no-op.
  - [ ] Right-click in overlay on a group is silently ignored (no edit dialog opens).
  - [ ] Escape inside group dialog: first press dismisses suggestions, second press cancels dialog.
