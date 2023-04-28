# Async Recorder

[![Docs](https://docs.rs/async-recorder/badge.svg)](https://docs.rs/async-recorder/)
[![Licence](https://img.shields.io/crates/l/async-recorder)](https://github.com/pacman82/async-recorder/blob/main/License)
[![Crates.io](https://img.shields.io/crates/v/async-recorder)](https://crates.io/crates/async-recorder)

Store records without waiting for your persistence backend. Load records without blocking writes.

A Rust crate for asynchronously recording events (or any other thing) with your persistence backend in a "fire and forget" manner. This crate uses the `tokio` runtime for spawning and joining green threads. As such it does not work with other asynchronous runtimes, such as `async-std`.
