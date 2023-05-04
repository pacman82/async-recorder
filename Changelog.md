# Changelog

## 0.3.1

* Added implementations for `Box<dyn Storage<..>>` in order to support scenarios there you want to choose the backend at runtime.

## 0.3.0

* Adds `from_delayed_storage` to support fast boot times for applications using recorder.
* `Recorder::new` is now synchronous.

## 0.2.0

Introduce `Storage::Query` to enabling push down filtering.

## 0.1.2

Support for read operation returning all records.

## 0.1.1

Initial release to crates.io
