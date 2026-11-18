import? '.toolchain/rust.just'

# Evaluate the shared MSRV only when a recipe uses it.
set lazy

_list:
    @just --list

# Check formatting and lint the library and examples.
check: && clippy
    just --check --fmt
    nixpkgs-fmt --check flake.nix
    fd --type=file --hidden --extension=toml --exec-batch taplo format --check
    cargo fmt --all -- --check
    just rdme --check

# Check security advisories.
check-advisories:
    cargo deny --log-level warn --workspace --all-features check advisories

# Run clippy on workspace.
clippy:
    cargo clippy --workspace --all-targets --all-features
    cargo hack --workspace clippy --all-targets --feature-powerset

# Format project files.
fmt:
    just --fmt
    nixpkgs-fmt flake.nix
    fd --type=file --hidden --extension=toml --exec-batch taplo format
    cargo fmt --all
    @just rdme --force

# Generate or check the README from crate root documentation.
rdme *args:
    cargo rdme {{ args }}

# Run the test suite with each TLS backend.
test:
    cargo test
    cargo test --no-default-features --features ring
