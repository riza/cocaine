<p align="center">
  <img src="assets/logo.png" width="200" alt="cocaine logo">
</p>

<h1 align="center">cocaine</h1>

<p align="center">
  Keep your machine awake. Like <code>caffeinate</code>, but with more kick.<br>
  A tiny Rust CLI that prevents your computer from sleeping. Works on macOS, Linux and Windows.
</p>

## Install

### Homebrew (macOS/Linux)

```bash
brew install riza/tap/cocaine
```

### From source

```bash
cargo install --git https://github.com/riza/cocaine.git
```

### From release

Download the latest binary for your platform from [Releases](https://github.com/riza/cocaine/releases), extract, and put it in your `$PATH`.


| Platform              | Binary                                     |
| --------------------- | ------------------------------------------ |
| macOS (Apple Silicon) | `cocaine-aarch64-apple-darwin.tar.gz`      |
| macOS (Intel)         | `cocaine-x86_64-apple-darwin.tar.gz`       |
| Linux (x86_64)        | `cocaine-x86_64-unknown-linux-gnu.tar.gz`  |
| Linux (ARM64)         | `cocaine-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64)      | `cocaine-x86_64-pc-windows-msvc.zip`       |


## Usage

```
cocaine [OPTIONS] [COMMAND]...
```

By default (no flags), cocaine prevents idle system sleep indefinitely until you hit Ctrl+C.

### Options


| Flag | Long               | Description                                    |
| ---- | ------------------ | ---------------------------------------------- |
| `-d` | `--display`        | Prevent the display from sleeping              |
| `-i` | `--idle`           | Prevent the system from idle sleeping          |
| `-s` | `--system`         | Prevent the system from sleeping entirely      |
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

## How it works

cocaine uses native OS APIs on each platform to prevent sleep:


| Platform    | API                                 | Mechanism                                                     |
| ----------- | ----------------------------------- | ------------------------------------------------------------- |
| **macOS**   | IOKit `IOPMAssertionCreateWithName` | Creates power assertions that are released on exit            |
| **Linux**   | systemd-logind D-Bus `Inhibit`      | Holds an inhibit file descriptor via `org.freedesktop.login1` |
| **Windows** | `SetThreadExecutionState`           | Sets `ES_SYSTEM_REQUIRED` / `ES_DISPLAY_REQUIRED` flags       |


Assertions / inhibitors are automatically released when cocaine exits -- whether that's through Ctrl+C, a timeout, or a child command finishing.

### Verifying

```bash
# macOS
pmset -g assertions

# Linux
systemd-inhibit --list

# Windows (PowerShell)
powercfg /requests
```

## License

[MIT](LICENSE)