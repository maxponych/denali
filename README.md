# Denali

⚠️ **Work in progress** - functional but not production ready

Snapshot-based backup tool with granular control. Create named snapshots of your entire project or specific components ("cells") within it.

## Why Denali?

Traditional backups are all-or-nothing. Denali lets you snapshot different parts of your project independently. Working on an OS? Snapshot your kernel, drivers, and libraries separately. Each cell maintains its own history while staying linked to your project.

**Key features:**
- Named snapshots for projects and cells
- Git-like object storage (automatic deduplication)
- Cells can live anywhere on your filesystem
- Restore entire projects or individual cells
- Time-based filtering (restore newest snapshot before/after a date)

## Quick Start
```bash
# Initialize a project
denali init myproject -p /path/to/project

# Create a snapshot
denali save myproject "initial setup"

# Initialize a cell within the project
denali init kernel@myproject -p /path/to/kernel

# Snapshot just the cell
denali save kernel@myproject "working version"

# List everything
denali list all

# Load a snapshot
denali load myproject "initial setup"
```

## Commands

### `denali init <name> -p <path> [-d <description>]`
Initialize a project or cell.
- Use `project_name` for projects
- Use `cell@project` for cells

### `denali save <name> <snapshot_name> [-d <description>]`
Create a named snapshot.

### `denali load <name> [snapshot_name] [options]`
Restore a snapshot.
- `-p <path>` / `--path <path>` - restore to custom location
- `-b <date>` / `--before <date>` - load newest before this time
- `-a <date>` / `--after <date>` - load newest after this time
- `-c` / `--with-config` - include .denali.toml config file
- `-f <path>` / `--from <path>` - load from alternate .denali directory

### `denali copy <name> -p <path>`
Export project/cell to specified directory (use `all` to copy everything).

### `denali list <name>`
List projects, cells, or snapshots (use `all` to list everything).

## Config file -  `.denali.toml`
The config file will be generated on project initialisation inside the project root. In this file you can specify ignore, filters, locks, name, description, path. Config will not affect load if load is called with `--path` argument.

Example:
```toml
[root]
name = "os"
description = "My toy OS"
ignore = ["*.bin"]

[libk]
description = "My library"
path = "/home/user/projects/os/libk"
ignore = ["*.bin", "*.elf", "*.o", "src/*.o"]
snapshot_before = "20-03-2025" 

[kernel]
description = "My kernel"
path = "/home/user/projects/os/kernel"
ignore = ["*.bin", "*.elf", "*.o"]
lock = "stable"

[drivers]
description = "My drivers"
path = "/home/user/projects/os/drivers"
ignore = ["*.bin", "*.elf", "*.o", "src/*.o"]
snapshot_after = "20-05-2025 13:11"
```
At the moment only ignore, snapshot_before/after, lock affect the load. Changing the name, description or path in config does not affect the commands *yet*. `check` command, when done, is going to scan the config and change the values in manifests respectively.

### `lock = "<name>"`
Locks this particular cell at the specified snapshot. Denali will ignore any values passed into `load` command for that cell if lock is set. 

### `ignore = ["<rule>", "<rule>"]`
Ignore rules for your project. *Note that cell ignore rules are relative to cell path*.

### `snapshot_before/after = "<date>"`
Filter for snapshots. `load` will load newest within specified constrains.

## Current Limitations

- Local storage only (remote sync planned)
- No symlink support yet
- File permissions not preserved
- Missing: cleanup commands, diff, merge strategies

## Roadmap

- [ ] `check` command (manifest synchronisation with config file)
- [ ] Templates
- [ ] Remote sync (push/pull)
- [ ] Diff command
- [ ] Snapshot cleanup/pruning
- [ ] Merge strategies

## Contributing

Built by a 16-year-old learning Rust - code might be rough in places, PRs welcome!

## License

Licensed under the [MIT License](./LICENSE).
