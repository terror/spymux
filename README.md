## spymux

**spymux** is a terminal user-interface for simultaneously watching all of your
open [tmux](https://github.com/tmux/tmux) panes.

<img width="1337" alt="demo" src="screenshot.png" />

Why? I pair program a fair amount with tools like [codex](https://github.com/openai/codex),
and I run them in different projects at the same time. I'd like a tool that gives
me a clear view into what the agents are doing, at the same time.

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
spymux 0.1.0

A centralized view for all of your tmux panes

Usage: spymux [OPTIONS] [COMMAND]

Commands:
  resume  Resume a running spymux instance via fzf
  help    Print this message or the help of the given subcommand(s)

Options:
  -n, --no-colors  Disable colored output
  -h, --help       Print help
  -V, --version    Print version
```

## Prior Art

This project is loosely inspired by tools like [Claude Squad](https://github.com/smtg-ai/claude-squad). I want less of the management aspect, and more of a simple view into how things are going
across the various agent instances I have open at any given time.
