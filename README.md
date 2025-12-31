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
- Permissions and symlinks support

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
Each command accepts a `--root <path>` argument, which specifies the path to the object store. For example, if your object store is located at `usb/projects/.denali`, you should set `--root` to the directory containing it, e.g. `--root usb/projects`.

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
- `-w` / `--wipe` - wipe the destination directory

### `denali copy <name> -p <path>`
Export project/cell to specified directory (use `all` to copy everything).

### `denali list <name>`
List projects, cells, or snapshots (use `all` to list everything).

### `denali remove <name> [snapshot_name] [--all]`
Remove snapshot and project/cell from the manifests.
Use `--all` only when deleting a snapshot from a project to remove it from all cells.
In order to clean up you still need to call `denali clean`

### `denali clean [--dry]`
Clean detached objects. `--dry` is going to return hashes of snapshots metadata that is going to be removed.

### `denali check [-p <path>]`
Compare config file with manifests. `-p` must point to the directory containing the `denali.toml` file.

### `denali tmpl new <name> [-p <path> --override]`
Create a template. Use override in case you want to change the existing one.

### `denali tmpl apply <name> [-p <path> --dry --with_config]`
Load a template. Load the structure of template and any commands provided. `--dry` is in case you don't want to execute commands for that template, but structure will be restored. 

### `denali tmpl list`
List templates

### `denali tmpl remove <name>`
Remove template from manifests. In order to clean up you still need to call `denali clean`

### `denali sync <name> <remote>`
Sync storage and manifests with remote host (use `all` to sync all projects). Requirements for host is accept SSH connections and have denali installed. SSH uses `BatchMode=yes` so authentication to the host must be established beforehand.

### `denali remote add <name> <host>`
Add remote to remotes list. Format for host `user@host:/path/to/use`.

### `denali remote remove <name>`
Remove remote from remotes list.

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
In order to rename cell, just change the table name. `check` command will then compare the path to detect name change.

### `name = <name>`
Project name. *Note that value is available only in `root` table*.

### `description = "<description>"`
Your project/cell description

### `path = "<path>"`
Path to your cell. *Note that it must be absolute*

### `lock = "<name>"`
Locks this particular cell at the specified snapshot. Denali will ignore any values passed into `load` command for that cell if lock is set. 

### `ignore = ["<rule>", "<rule>"]`
Ignore rules for your project. *Note that cell ignore rules are relative to cell path*.

### `snapshot_before/after = "<date>"`
Filter for snapshots. `load` will load newest within specified constrains.

## Templates
Templates are your ready to use development environment setup. Directory tree and any commands you need to run.
The objects of you template are saved in the same store `.denali/objects`, the same store that is used for projects/cells objects.
To create template create a structure you want to use first. Inside the root directory you can create a config file with the name `.denali.tmpl.toml`. \
Example:
```toml
placeholders = ["path", "greetings"]
commands = ["cat <{path}>", "echo <{greetings}>", "npm i", "docker run something"]
```
Inside this config file you can add commands that will be executed on `denali tmpl apply`, after the structure was restored. You can also use placeholders for your commands. On apply you will be prompted to provide values for each placeholder. To use placeholder value in commands use `<{}>` wrapper. *Important: without any spaces*. \
If you later wish to change list of commands or placeholder, the config file is copied to the `.denali` store. So you can acces `.denali/templates/{template_name}.toml`, change it however you want.
Only `commands` and `placeholders` are supported for template configs at the moment.

## Current Limitations

- Missing: diff, merge strategies

## Roadmap

- [x] `check` command (manifest synchronisation with config file)
- [x] Templates
- [x] Remote sync (push/pull)
- [ ] Diff command
- [x] Snapshot cleanup/pruning
- [ ] Merge strategies for snapshots

## Contributing

Built by a 16-year-old learning Rust - code might be rough in places, PRs welcome!

## License

Licensed under the [MIT License](./LICENSE).
