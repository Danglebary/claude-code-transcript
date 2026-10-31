set positional-arguments

# The prefix that puts a command inside the flake's shell: nothing where the shell's
# marker `CLAUDE_CODE_TRANSCRIPT_SHELL` is set, so a command already inside it runs in
# place, and `nix develop` everywhere else, since a plain shell resolves the machine's
# toolchain rather than the project's.
in_shell := if env("CLAUDE_CODE_TRANSCRIPT_SHELL", "") == "1" { "" } else { "nix develop --command" }

# The gate.
check:
    {{in_shell}} cargo fmt --all --check
    {{in_shell}} cargo clippy --all-targets -- -D warnings
    {{in_shell}} cargo test
