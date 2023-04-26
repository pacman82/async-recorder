# Async Recorder

A rust crate for asynchronously recording events (or any other thing) with your persistence backend.

This crate uses the `tokio` runtime for spawning and joining green threads. As such it does not work with other asynchronous runtimes, such as `async-std`.
