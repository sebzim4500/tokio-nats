# Changelog

## Unreleased

- The `Error::source()` trait method now returns error sources where applicable.
- Update `rustls` dependency to `0.23`.
- Remove `aws_lc_rs` feature alias, use `aws-lc-rs`.
- Remove `aws-lc-rs` from default features.
- Removing `rustls-pemfile` dependency.
- Upgrade to Rust edition 2024.
- Minimum supported Rust version (MSRV) is now 1.95.

## 0.4.2

- The `NatsClient::publish()` and `NatsClient::subscribe()` methods now take `&self`.
