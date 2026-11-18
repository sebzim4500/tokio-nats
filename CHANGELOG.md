# Changelog

## Unreleased

- Replace `derive_more` with `impl-more`. `Error::source()` now returns the wrapped I/O or UTF-8 error.
- Update `parking_lot` to 0.13 and set the minimum supported Rust version to 1.95.
- Move to Rust edition 2024.
- Declare rustls 0.23 as a direct dependency.
- Use rustls to read TLS certificates and private keys in PEM format, removing the `rustls-pemfile` dependency.
- Remove `aws_lc_rs` feature alias, use `aws-lc-rs`.
- Remove `aws-lc-rs` as a default feature.

## 0.4.2

- `NatsClient::publish` and `NatsClient::subscribe` now take `&self`.
