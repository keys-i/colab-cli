//! Build-time embedded OAuth credentials.
//!
//! The actual implementation lives in `$OUT_DIR/embedded_secrets.rs`, which is
//! produced by `build.rs`. It exposes two helpers that return the OAuth
//! `client_id` / `client_secret` baked into the binary at compile time:
//!
//! - `embedded_client_id()`
//! - `embedded_client_secret()`
//!
//! Both return obfuscated strings via `obfstr`, so the plaintext never lands
//! in `.rodata`. When the build-time env vars were empty, they return `""`
//! and `config.rs` falls back to the runtime env / `config.toml` lookup.

include!(concat!(env!("OUT_DIR"), "/embedded_secrets.rs"));
