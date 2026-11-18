# Changelog

## Unreleased

- Replace `derive_more` with `impl-more`. `Error::source()` now returns the wrapped I/O or UTF-8 error.
- Update `parking_lot` to 0.13, which requires Rust 1.95 or later.
- Declare rustls 0.23 as a direct dependency.

## 0.4.2

- `NatsClient::publish` and `NatsClient::subscribe` now take `&self`.
