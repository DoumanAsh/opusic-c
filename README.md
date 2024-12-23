# opusic-c

[![Rust](https://github.com/DoumanAsh/opusic-c/actions/workflows/rust.yml/badge.svg)](https://github.com/DoumanAsh/opusic-c/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/opusic-c.svg)](https://crates.io/crates/opusic-c)
[![Documentation](https://docs.rs/opusic-c/badge.svg)](https://docs.rs/crate/opusic-c/)

High level bindings to [libopus](https://github.com/xiph/opus)

Target version [1.5.2](https://github.com/xiph/opus/releases/tag/v1.5.2)

## Allocator

This library uses Rust's allocator whenever possible

## Features

- `dred` - Enables experimental DRED decoder

## Setup

If the `OPUS_LIB_DIR` environment variable is set, it will be searched for the opus library.
Otherwise, a static library will be built automatically.

## Android build

When building for android, library requires presence of env variable `ANDROID_NDK_HOME` in order to supply
cmake with toolchain file and correct target arch.

## Requirements

- `cmake`

### Optional

- `ninja` - When present, build script defaults to use corresponding CMake's generator
