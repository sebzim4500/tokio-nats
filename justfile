import '.toolchain/rust.just'

# Evaluate the shared MSRV only when a recipe uses it.
set lazy

_list:
    @just --list

# Check formatting and lint the library and examples.
check:
    just --check --fmt
    nixpkgs-fmt --check flake.nix
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features

# Format project files.
fmt:
    just --fmt
    nixpkgs-fmt flake.nix
    cargo fmt --all

# Run the test suite with each TLS backend.
test:
    cargo test
    cargo test --no-default-features --features ring
