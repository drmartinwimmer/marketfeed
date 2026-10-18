# marketfeed

[![CI](https://github.com/drmartinwimmer/marketfeed/actions/workflows/ci.yml/badge.svg)](https://github.com/drmartinwimmer/marketfeed/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/marketfeed.svg)](https://crates.io/crates/marketfeed)
[![Docs.rs](https://docs.rs/marketfeed/badge.svg)](https://docs.rs/marketfeed)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![MSRV: 1.88+](https://img.shields.io/badge/MSRV-1.88%2B-blue.svg?logo=rust)](https://www.rust-lang.org/)

Typed Rust client library and CLI for financial market and currency data sources.

## Overview

`marketfeed` provides a clean, stateless Rust interface and CLI binary for querying and parsing financial market data across multiple online APIs and file-based exchange exports. It returns strongly typed Rust structs directly from each backend without opinionated caching, storage, or domain transformations—while transparently handling multi-page pagination where applicable.

- **Direct Typed Backend Responses**: Deserializes backend responses directly into Rust structs using `rust_decimal::Decimal` and `chrono` timestamps.
- **Transparent Pagination**: Automatically handles offset/page iteration for paginated APIs (such as Marketstack) with configurable item, page, or full-fetch limits.
- **Stateless & Composable**: Pure data-source client library and CLI without local database, mirror, or updater coupling.

## Installation

### Library

```toml
[dependencies]
marketfeed = "0.1"
```

### CLI

```bash
cargo install marketfeed
```

## License

MIT
