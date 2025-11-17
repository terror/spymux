## spymux

[![release](https://img.shields.io/github/release/terror/spymux.svg?label=release&style=flat&labelColor=282c34&logo=github)](https://github.com/terror/spymux/releases/latest)
[![crates.io](https://shields.io/crates/v/spymux.svg)](https://crates.io/crates/spymux)
[![CI](https://github.com/terror/spymux/actions/workflows/ci.yaml/badge.svg)](https://github.com/terror/spymux/actions/workflows/ci.yaml)
[![codecov](https://codecov.io/gh/terror/spymux/graph/badge.svg?token=7CH4XDXO7Z)](https://codecov.io/gh/terror/spymux)
[![downloads](https://img.shields.io/github/downloads/terror/spymux/total.svg)](https://github.com/terror/spymux/releases)
[![dependency status](https://deps.rs/repo/github/terror/spymux/status.svg)](https://deps.rs/repo/github/terror/spymux)

**spymux** is a terminal user-interface for simultaneously watching all of your
open [tmux](https://github.com/tmux/tmux) panes.

<img width="1337" alt="demo" src="screenshot.png" />

Why? I pair program a fair amount with tools like [codex](https://github.com/openai/codex),
and I run them in different projects (windows) at the same time. I'd like a tool that gives
me a clear view into what the agents are doing, without having to switch between them.

## Installation

`spymux` should run on any system, including Linux, MacOS, and the BSDs.

The easiest way to install it is by using
[cargo](https://doc.rust-lang.org/cargo/index.html), the Rust package manager:

```bash
cargo install spymux
```

Otherwise, see below for the complete package list:

#### Cross-platform

<table>
  <thead>
    <tr>
      <th>Package Manager</th>
      <th>Package</th>
      <th>Command</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><a href=https://www.rust-lang.org>Cargo</a></td>
      <td><a href=https://crates.io/crates/spymux>spymux</a></td>
      <td><code>cargo install spymux</code></td>
    </tr>
    <tr>
      <td><a href=https://brew.sh>Homebrew</a></td>
      <td><a href=https://github.com/terror/homebrew-tap>terror/tap/spymux</a></td>
      <td><code>brew install terror/tap/spymux</code></td>
    </tr>
  </tbody>
</table>

## Usage

**spymux** is very simple to use, once installed you should be able to invoke the
binary in any [tmux](https://github.com/tmux/tmux) session and have it work:

```
spymux
```

We support a few configuration options, as seen below:

```present cargo run -- --help
spymux 0.1.1

A centralized view for all of your tmux panes

Usage: spymux [OPTIONS] [COMMAND]

Commands:
  resume  Resume a spymux session in another directory
  help    Print this message or the help of the given subcommand(s)

Options:
  -n, --no-colors                    Disable colored output
      --refresh-rate <MILLISECONDS>  Refresh interval in milliseconds (default: 500)
  -h, --help                         Print help
  -V, --version                      Print version
```

### Keybindings

| Action | Keys |
| --- | --- |
| Move up | ↑ / `k` |
| Move down | ↓ / `j` |
| Move left | ← / `h` |
| Move right | → / `l` |
| Focus highlighted pane | `enter` |
| Hide highlighted pane | `x` |
| Quit spymux | `q` / `esc` |
| Toggle help | `?` |
| Select clicked pane | left click |

## Prior Art

This project is loosely inspired by tools like [Claude Squad](https://github.com/smtg-ai/claude-squad). I want less of the management aspect, and more of a simple view into how things are going
across the various agent instances I have open at any given time.
