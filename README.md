# cocaine

Keep your Mac awake. Like `caffeinate`, but with more kick.

A tiny Rust CLI that prevents your Mac from sleeping using macOS power assertions (`IOPMAssertion`). No dependencies beyond the system frameworks.

## Install

### Homebrew

```bash
brew install riza/tap/cocaine
```

### From source

```bash
cargo install --git https://github.com/riza/cocaine.git
```

### From release

Download the latest binary from [Releases](https://github.com/riza/cocaine/releases), extract, and put it in your `$PATH`.

## Usage

```
cocaine [OPTIONS] [COMMAND]...
```

By default (no flags), cocaine prevents idle system sleep indefinitely until you hit Ctrl+C.

### Options

| Flag | Long | Description |
|------|------|-------------|
| `-d` | `--display` | Prevent the display from sleeping |
| `-i` | `--idle` | Prevent the system from idle sleeping |
| `-s` | `--system` | Prevent the system from sleeping entirely |
| `-t` | `--timeout <SECS>` | Stop after N seconds (default: 0 = indefinite) |

### Examples

```bash
# Prevent idle sleep until Ctrl+C
cocaine

# Keep the display on
cocaine -d

# Keep the display on for 1 hour
cocaine -d -t 3600

# Prevent all sleep types
cocaine -d -i -s

# Keep awake while a command runs
cocaine -d -- make build

# Keep awake during a long download
cocaine -s -- curl -O https://example.com/big-file.tar.gz
```

### How it works

cocaine creates macOS `IOPMAssertion`s through the IOKit framework:

- `-d` creates a `PreventUserIdleDisplaySleep` assertion
- `-i` creates a `PreventUserIdleSystemSleep` assertion (default when no flags given)
- `-s` creates a `PreventSystemSleep` assertion

Assertions are automatically released when cocaine exits (via `Drop`), whether that's through Ctrl+C, a timeout, or a child command finishing.

You can verify active assertions with:

```bash
pmset -g assertions
```

## Requirements

- macOS (uses IOKit framework)

## Release

Tag a version to trigger the release workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This builds universal macOS binaries (ARM + Intel), creates a GitHub release, and updates the Homebrew formula automatically.

## License

[MIT](LICENSE)
