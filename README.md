# [hyperion.rs](https://github.com/vtavernier/hyperion.rs)

[![Build](https://github.com/vtavernier/hyperion.rs/workflows/build/badge.svg)](https://github.com/vtavernier/hyperion.rs/actions)
[![Docs](https://img.shields.io/badge/docs-master-blue)](https://vtavernier.github.io/hyperion.rs/hyperion/)
[![GitHub](https://img.shields.io/github/license/vtavernier/hyperion.rs)](LICENSE)

hyperion.rs is a rewrite of
[hyperion.ng](https://github.com/hyperion-project/hyperion.ng) in the Rust
Programming Language. This version features:

* Very low resource requirements (can be run on a single thread, useful for the
  Raspberry Pi Zero for example)
* No memory leaks (most allocations are static anyways)
* Easy to compile *and* cross-compile

***Disclaimer: this is an early work-in-progress***:
* A lot of internals may still change, in case you would like to contribute
  please contact me beforehand.
* Only a small subset of the original project's features are currently
  supported. This means that unless you're using it in the exact same context as
  I am (Android TV Grabber + Raspberry Pi Zero W + WS2815 LED strip), this might
  not be for you.

## Compatibility

Currently implemented features:

* Loading settings from the hyperion.ng database
* Basic WS2812SPI device (no invert, no latch time)
* JSON, Protobuf, Flatbuffers and Boblight server
* Black border detector, color channel adjustments, smoothing

## Configuration

### Migrating your settings

This rewrite uses the same database format for storing settings. In order to
load your existing hyperion.ng settings, assuming you are in your home
directory:

```bash
# Create config directory for hyperion.rs
$ mkdir -p .config/hyperion.rs/

# Copy the existing hyperion.ng database to the new location for hyperion.rs
$ cp .hyperion/db/hyperion.db .config/hyperion.rs/
```

### Using a TOML file

You may also configure hyperion.rs using a TOML representation of the configuration. To generate the initial file, you can use the `--dump-config` option:

```bash
$ hyperiond-rs --dump-config >config.toml
```

Then, you can start the daemon using this config file:

```bash
$ hyperiond-rs --config config.toml
```

The minimal configuration required is as follows:

```toml
[instances.0.instance]
friendlyName = 'Test instance'
enabled = true

[instances.0.device]
type = 'dummy'
```

## Running hyperion.rs

Once your settings database has been migrated, you can run hyperion.rs using
`cargo`:

```bash
$ cargo run
```

If running from a release archive, invoke the `hyperiond-rs` binary directly.

## Cross-compiling

Cross-compiling is done using [cross](https://github.com/rustembedded/cross).
Let's say we are building for the Raspberry Pi Zero, which corresponds to the
Rust target arm-unknown-linux-gnueabihf.

```bash
$ export TARGET=arm-unknown-linux-gnueabihf

# (if not done already) Install cross
$ cargo install --force cross

# Build the project
$ cross build --release --target $TARGET

# The resulting binaries will be in target/$TARGET/release
```

## License

This work is licensed under the [MIT License](LICENSE).

## Author

Vincent Tavernier <vince.tavernier@gmail.com>. Original project and protocol
source files by [hyperion-project](https://github.com/hyperion-project).
