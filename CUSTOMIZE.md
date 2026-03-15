# todo — Customization Guide

A minimal task manager harness. Ships with JSON file storage, plain-text CLI output,
and a clean web UI served over HTTP. Does exactly four things: add, complete, delete,
list. Everything else — categories, priorities, syncing, rich formatting — is a port
waiting to be filled.

## Ports

### `Storage`

**What it does:** Controls where tasks are persisted.
**Default:** `JsonStorage` — writes a JSON array to `~/.config/todo/tasks.json`.
**How to customize:** Implement the `Storage` trait in `src/storage.rs`:

```rust
pub trait Storage {
    fn load(&self) -> Result<Vec<Task>>;
    fn save(&self, tasks: &[Task]) -> Result<()>;
}
```

Then swap `JsonStorage::default()` for your type in `src/main.rs`:

```rust
// Before
let storage = JsonStorage::default();

// After (e.g. SQLite)
let storage = SqliteStorage::new("/path/to/tasks.db");
```

Common swaps: SQLite, a remote HTTP API, an in-memory store for testing.

---

### `Formatter`

**What it does:** Controls how tasks are rendered in CLI output (`todo list`).
**Default:** `PlainText` — renders `[ ] #1: Buy groceries` style lines.
**How to customize:** Implement the `Formatter` trait in `src/formatter.rs`:

```rust
pub trait Formatter {
    fn format(&self, tasks: &[Task]) -> String;
}
```

Then swap `PlainText` for your type in `src/main.rs`:

```rust
// Before
let formatter = PlainText;

// After (e.g. color-coded by done state)
let formatter = ColorFormatter;
```

---

### `Lifecycle Hooks`

**What it does:** Runs shell scripts after add, complete, and delete operations.
**Default:** No hooks — the `hooks/` directory ships empty.
**How to customize:** Drop executable shell scripts into `hooks/`:

| File | Fires when | Args |
|------|-----------|------|
| `hooks/on_add.sh` | A task is added | `$1` = id, `$2` = text |
| `hooks/on_complete.sh` | A task is marked done | `$1` = id, `$2` = text |
| `hooks/on_delete.sh` | A task is deleted | `$1` = id, `$2` = text |

Scripts that don't exist are silently skipped. Scripts must be executable (`chmod +x`).

Example — send an SMS when a task is added:

```sh
#!/bin/sh
# hooks/on_add.sh
curl -s -X POST http://your-txtme-instance/send \
  -d "message=New task: $2"
```

---

### `Web UI`

**What it does:** The HTML served at `/` when running `todo serve`.
**Default:** `src/todo.html` — a minimal dark-theme task list, embedded at compile time.
**How to customize:** Edit `src/todo.html` directly. The server injects task data as
JSON by replacing the `__TASKS_JSON__` placeholder before serving:

```html
<script>
  const tasks = __TASKS_JSON__;  // replaced at request time
</script>
```

Add categories, drag-to-reorder, priority indicators, or any other UI — this file is
yours. Rebuild after changes (`cargo build --release`).

## Getting Started

```sh
git clone https://github.com/Slam-Dunk-Software/todo
cd todo
cargo build --release

# CLI usage
./target/release/todo add "Buy groceries"
./target/release/todo list
./target/release/todo done 1
./target/release/todo delete 1

# Web UI
./target/release/todo serve --port 8765
# open http://localhost:8765
```

## Common Customizations

### Example: Adding a category field to tasks

1. Add a `category: Option<String>` field to `Task` in `src/task.rs`
2. Update `AddTask` in `src/serve.rs` to accept a `category` form field
3. Update `src/todo.html` to render a category badge and include the field in the add form

### Example: Sending a notification on task completion

```sh
#!/bin/sh
# hooks/on_complete.sh
osascript -e "display notification \"Done: $2\" with title \"Todo\""
```

### Example: Priority-sorted output

Implement a custom `Formatter` that sorts tasks by a priority prefix in the text
(e.g. `!high buy groceries`) before rendering.
