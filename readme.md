# dirit

A simple CLI tool for editing and applying filesystem changes through your text editor.

## Features

* Edit paths in your `$EDITOR` / `$VISUAL`
* Rename files and directories
* Copy files
* Create files and directories
* Remove paths
* Recursive mode
* Dry-run mode

## Installation

```bash
cargo install --path .
```

## Usage

```bash
dirit
dirit file.txt
dirit -r directory/
dirit --dry-run
```

`dirit` opens the selected paths in your configured editor. Edit the paths, save the file, and exit the editor to apply the changes.

## License

MIT
