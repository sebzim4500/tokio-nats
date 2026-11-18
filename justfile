import '.toolchain/rust.just'

# Evaluate the shared MSRV only when a recipe uses it.
set lazy

_list:
    @just --list

# Check the library and examples.
check:
    just --check --fmt
    nixpkgs-fmt --check flake.nix
    cargo check --all-targets --all-features

# Format project files.
fmt:
    just --fmt
    nixpkgs-fmt flake.nix
    cargo fmt --all

# Run the test suite.
test:
    cargo test
