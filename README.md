# Async Recorder

Store records without waiting for your persistence backend.

A Rust crate for asynchronously recording events (or any other thing) with your persistence backend in a "fire and forget" manner. This crate uses the `tokio` runtime for spawning and joining green threads. As such it does not work with other asynchronous runtimes, such as `async-std`.
