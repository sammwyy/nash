use super::Builtin;
use crate::runtime::{Context, Output};
use anyhow::Result;

pub struct Help;

const HELP_TEXT: &str = r#"Nash — Not A Shell  |  Sandboxed bash-like interpreter

AVAILABLE COMMANDS
──────────────────────────────────────────────────────────────
  cat    [-n] [file...]       Print file(s) or stdin
  cd     [dir]                Change working directory
  clear                       Clear the terminal screen
  cp     SRC DST              Copy file
  cut    -d D -f N [file...]  Cut fields from lines
  echo   [-n] [-e] [text...]  Print text
  env                         List environment variables
  export KEY=VALUE            Set environment variable
  false                       Exit with code 1
  file   [path...]            Detect file type
  find   [path] [options]     Search for files
    -name PATTERN               Match filename glob
    -type f|d                   Filter by type
    -maxdepth N                 Limit depth
  grep   [-v] [-i] [-n] PATTERN [file...]  Filter lines
  head   [-n N] [file...]     First N lines (default 10)
  help   [command]            Show this help
  history [N]                 Show command history
  jq     FILTER [file]        Process JSON data
  ls     [-l] [-a] [path...]  List directory contents
  mkdir  [-p] DIR...          Create directory
  mv     SRC DST              Move or rename file
  pwd                         Print working directory
  rm     [-r] [-rf] PATH...   Remove file or directory
  sed    EXPR [file...]       Stream editor
    s/old/new/[g]               Substitute
    Nd                          Delete line N
  sort   [-r] [-u] [file...]  Sort lines
  stat   [path...]            File status info
  tail   [-n N] [file...]     Last N lines (default 10)
  test   EXPR / [ EXPR ]      Evaluate expression
    -f FILE   true if regular file
    -d FILE   true if directory
    -e FILE   true if exists
    -z STR    true if empty string
    -n STR    true if non-empty string
    = != -eq -ne -lt -le -gt -ge
  touch  FILE...              Create empty file
  tree   [-L N] [-a] [path]   Directory tree view
  true                        Exit with code 0
  uniq   [-c] [-d] [-u]       Filter adjacent duplicates
  unset  VAR...               Unset environment variable
  wc     [-l] [-w] [-c]       Count lines/words/bytes
  which  CMD...               Show command location

SYNTAX
──────────────────────────────────────────────────────────────
  cmd arg              Simple command
  cmd | cmd            Pipe stdout to stdin
  cmd > file           Redirect stdout (overwrite)
  cmd >> file          Redirect stdout (append)
  cmd < file           Read stdin from file
  cmd && cmd           Run second if first succeeds
  cmd || cmd           Run second if first fails
  cmd ; cmd            Run both unconditionally
  ( cmd )              Subshell (isolated environment)
  $VAR  ${VAR}         Variable expansion
  $(cmd)               Command substitution
  'text'  "text"       Quoting (single: no expansion)
  # comment            Line comment

Type 'exit' or press Ctrl-D to quit.
"#;

impl Builtin for Help {
    fn run(&self, _args: &[String], _ctx: &mut Context, _stdin: &str) -> Result<Output> {
        Ok(Output::success(HELP_TEXT))
    }
}
