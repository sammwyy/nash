<div align="center">

```
███╗   ██╗ █████╗ ███████╗██╗  ██╗
████╗  ██║██╔══██╗██╔════╝██║  ██║
██╔██╗ ██║███████║███████╗███████║
██║╚██╗██║██╔══██║╚════██║██╔══██║
██║ ╚████║██║  ██║███████║██║  ██║
╚═╝  ╚═══╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
```

**Not A Shell** — a sandboxed, bash-like command interpreter written in Rust.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Builtins](https://img.shields.io/badge/builtins-28-purple?style=flat-square)]()

</div>

---

Nash looks and behaves like a minimal Bash shell, but it **never executes real system commands or touches the host filesystem directly**. Everything runs inside a fully controlled in-memory Virtual Filesystem (VFS) with an optional host-directory overlay via explicit mount bindings.

```bash
user@nash:/home/user$ ls
Desktop/  Documents/  Downloads/  welcome.txt

user@nash:/home/user$ echo "hello world" | grep hello
hello world

user@nash:/home/user$ mkdir projects && cd projects
user@nash:/home/user/projects$ pwd
/home/user/projects
```

## Features

- **Bash-like syntax** — pipes, redirections, `&&`, `||`, `;`, subshells, quoting, `$VAR`, `$(cmd)`
- **Full sandbox** — zero `std::process::Command` calls, no OS shell ever spawned
- **In-memory VFS** — realistic Unix directory tree scaffolded at boot
- **Host mounts** — bind real directories read-write or read-only via `--bind`
- **28 built-in commands** — `grep`, `sed`, `find`, `jq`, `tree`, `cut`, `uniq`, and more
- **Interactive REPL** — colored bash-style prompt, readline history, Ctrl-C/D handling
- **Script execution** — `nash script.sh` runs any `.sh` file directly
- **Multi-user** — `-u alice` creates `/home/alice` and sets all Unix env vars correctly

## Installation

```bash
git clone https://github.com/sammwyy/nash
cd nash
cargo build --release
```

The binary lands at `target/release/nash`. No dependencies beyond the Rust toolchain.

## Usage

### Interactive REPL

```bash
nash
nash -U alice
```

```
Nash — Not A Shell  │  logged in as user  │  type help or Ctrl-D to exit

user@nash:/home/user$
```

---

# Run a script

```bash
nash script.sh
nash ./deploy.sh
```

Shebangs are ignored:

```bash
#!/usr/bin/env nash
```

---

# One-shot command

```bash
nash -c "echo hello | grep hello"
```

---

# Read commands from stdin

```bash
cat commands.txt | nash
```

or

```bash
nash -s < commands.txt
```

---

# Mount host directories

Read-write mount:

```bash
nash -B ./project:/project
```

Read-only mount:

```bash
nash --bind-ro ./data:/data
```

Multiple mounts:

```bash
nash -B ./src:/src -B ./data:/data -C /src
```

---

# Set environment variables

```bash
nash -E DEBUG=true -E API_URL=http://localhost:8080
```

---

# Bash-compatible shell flags

| Flag | Meaning |
|-----|------|
| `-e` | exit on error |
| `-u` | error on unset variables |
| `-x` | print commands before executing |
| `-v` | print lines as they are read |
| `-i` | force interactive |
| `-l` | login shell |

Example:

```bash
nash -x script.sh
```

---

# RC Files

By default Nash loads:

```
/etc/profile
~/.nashrc
```

Disable rc loading:

```bash
nash --norc
```

Use a custom rc file:

```bash
nash --rcfile ./custom.nashrc
```

---

# CLI Reference

```
USAGE
    nash [OPTIONS] [SCRIPT]

POSITIONAL
    SCRIPT                 Script file to execute

SHELL FLAGS
    -c CMD                 Execute command string
    -i                     Force interactive mode
    -l, --login            Login shell
    -s                     Read commands from stdin
    -e, --errexit          Exit on error
    -u, --nounset          Error on unset variables
    -x, --xtrace           Print commands before executing
    -v, --verbose          Print lines as read

NASH FLAGS
    -U, --user NAME        Session username
    -C, --cwd PATH         Starting directory
    -E, --env KEY=VALUE    Set environment variable
    -B, --bind HOST:VFS    Mount host directory
        --bind-ro HOST:VFS Mount read-only directory

OPTIONS
        --rcfile FILE
        --norc
        --version
        --help
```

---

## Built-in Commands

| Command | Description |
|---------|-------------|
| `cat` | Print files or pass-through stdin |
| `cd` | Change directory (`cd -` for previous, `cd` for `$HOME`) |
| `clear` | Clear terminal screen |
| `cp` | Copy files |
| `cut` | Cut fields from lines (`-d`, `-f`, `-c`) |
| `echo` | Print text (`-n`, `-e` for escape sequences) |
| `env` | List environment variables |
| `export` | Set environment variable (`KEY=VALUE`) |
| `false` | Exit with code 1 |
| `file` | Detect file type (magic bytes + extension heuristic) |
| `find` | Search for files (`-name glob`, `-type f\|d`, `-maxdepth N`) |
| `grep` | Filter lines (`-v` invert, `-i` case, `-n` line numbers) |
| `head` | First N lines (`-n N`) |
| `help` | Full command and syntax reference |
| `history` | Show command history (optional `history N` to limit) |
| `jq` | Process JSON (`.key`, `keys`, `values`, `length`, `type`, `.[]`) |
| `ls` | List directory (`-l` long, `-a` hidden) |
| `mkdir` | Create directory (`-p` parents) |
| `mv` | Move or rename files |
| `pwd` | Print working directory |
| `rm` | Remove files or directories (`-r`, `-rf`) |
| `sed` | Stream editor (`s/old/new/[g]`, `Nd` delete line, `d` delete all) |
| `sort` | Sort lines (`-r` reverse, `-u` unique) |
| `stat` | File status (size, type, path) |
| `tail` | Last N lines (`-n N`) |
| `test` / `[` | Evaluate expressions (`-f`, `-d`, `-e`, `-z`, `-n`, `=`, `-eq`, …) |
| `touch` | Create empty file |
| `tree` | Directory tree view (`-L N` depth, `-a` hidden) |
| `true` | Exit with code 0 |
| `uniq` | Filter adjacent duplicates (`-c` count, `-d` dupes, `-u` unique) |
| `unset` | Unset environment variable |
| `wc` | Count lines/words/bytes (`-l`, `-w`, `-c`) |
| `which` | Show whether a command is a known Nash builtin |

## Syntax

```bash
# Simple command
echo hello world

# Pipe
cat file.txt | grep foo | sort | uniq

# Redirections
echo hello > out.txt
echo world >> out.txt
cat < in.txt

# Chaining
mkdir dist && cd dist
test -f config.json || echo "missing config"

# Sequence
echo start ; echo end

# Subshell (isolated environment — env changes don't escape)
(cd /tmp && ls)

# Variable expansion
echo $HOME
echo ${USER}@nash

# Command substitution
echo "Files: $(ls | wc -l)"
echo "CWD is $(pwd)"

# Quoting
echo 'no $expansion here'
echo "with $USER expansion"

# Comments
echo hello  # this is a comment
```

## Virtual Filesystem

Nash boots with a standard Unix directory tree entirely in memory:

```
/
├── bin/         sbin/
├── usr/
│   ├── bin/     sbin/     local/bin/
├── etc/
│   ├── hostname            shells
├── var/
│   ├── log/     tmp/
├── tmp/
├── lib/         lib64/
├── opt/
├── home/
│   └── <user>/
│       ├── Desktop/        Documents/        Downloads/
│       ├── .nashrc
│       └── welcome.txt
└── root/        proc/       dev/
```

Host directories are only accessible through **explicit mounts**:

```bash
nash --bind ./project:/project                   # read-write
nash --read-only-bind ./config:/etc/config       # read-only
```

## Default Environment

Nash injects these Unix-standard variables at startup. All are overridable with `-E`:

| Variable | Default value |
|----------|---------------|
| `USER` | username from `-u` (default: `user`) |
| `LOGNAME` | same as `USER` |
| `HOME` | `/home/<user>` |
| `SHELL` | `nash` |
| `TERM` | `xterm-256color` |
| `LANG` | `en_US.UTF-8` |
| `LC_ALL` | `en_US.UTF-8` |
| `PATH` | `/usr/local/bin:/usr/bin:/bin` |
| `PWD` | current working directory (synced by `cd`) |
| `OLDPWD` | previous directory (used by `cd -`) |
| `HOSTNAME` | `nash` |
| `SHLVL` | `1` |

## Architecture

```
src/
├── main.rs
├── cli.rs                  CLI flags, REPL, script runner
├── parser/
│   ├── mod.rs              parse() entrypoint + tests
│   ├── ast.rs              Expr, Word, WordPart, RedirectMode
│   └── lexer.rs            Hand-written tokenizer
├── runtime/
│   ├── executor.rs         AST walker — zero system calls
│   ├── context.rs          cwd + env + VFS + history
│   └── output.rs           Output { stdout, stderr, exit_code }
├── vfs/
│   ├── mod.rs              Virtual Filesystem API
│   ├── node.rs             FsNode (File | Directory)
│   ├── path.rs             normalize, join, parent, basename
│   └── mount.rs            MountPoint + MountOptions
└── builtins/
    ├── mod.rs              Builtin trait + dispatch table
    └── *.rs                One file per command (28 total)
```

The parser and runtime are **completely decoupled** — the parser produces an `Expr` tree and knows nothing about execution. The runtime walks the tree and knows nothing about syntax.

## Security

Sandboxing in Nash is structurally enforced, not just a policy:

- `std::process::Command` is **never imported** anywhere in the codebase
- All file I/O goes through the `Vfs` API — host paths are unreachable without a `--bind`
- Read-only mounts reject writes at the VFS layer, not the command layer
- Subshells run on a **cloned context** — env mutations don't escape `( )`

```bash
# Verify no system calls exist
grep -r "std::process\|Command::new\|bash -c" src/
# (no output)
```

## Tests

```bash
cargo test
```

| Module | Coverage |
|--------|----------|
| `parser/lexer.rs` | Tokenizer: quotes, escapes, operators, variables |
| `parser/mod.rs` | AST shape for every syntax form |
| `vfs/mod.rs` | Read, write, append, mkdir, rm, copy, mount |
| `vfs/path.rs` | normalize, join, parent, basename edge cases |
| `runtime/executor.rs` | 50+ end-to-end integration tests |

## License

MIT — see [LICENSE](LICENSE).

## Author

Made with ♥ by [Sammwy](https://github.com/sammwyy)