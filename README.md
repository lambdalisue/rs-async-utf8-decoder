[![crates.io](https://img.shields.io/crates/v/async-utf8-decoder.svg)](https://crates.io/crates/async-utf8-decoder)
[![dependency status](https://deps.rs/repo/github/lambdalisue/rs-async-utf8-decoder/status.svg)](https://deps.rs/repo/github/lambdalisue/rs-async-utf8-decoder)
[![docs.rs](https://docs.rs/async-utf8-decoder/badge.svg)](https://docs.rs/async-utf8-decoder)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Build](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/build.yml/badge.svg)](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/build.yml)
[![Test](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/test.yml/badge.svg)](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/test.yml)
[![Audit](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/audit.yml/badge.svg)](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/audit.yml)
[![codecov](https://codecov.io/gh/lambdalisue/rs-async-utf8-decoder/branch/main/graph/badge.svg?token=ghor9fqplN)](https://codecov.io/gh/lambdalisue/rs-async-utf8-decoder)

# async-utf8-decoder

## Asynchronous and incremental UTF-8 decoder

`async-utf8-decoder` crate provides `Utf8Decoder` which allows to convert any object which
implements `AsyncRead` trait into a string stream which implements `Stream` trait.

### Example

```rust
use futures::io;
use futures::channel::mpsc;
use async_utf8_decoder::Utf8Decoder;

let (mut tx, rx) = mpsc::unbounded::<io::Result<Vec<u8>>>();
let mut decoder = Utf8Decoder::new(rx.into_async_read());

tx.send(Ok(vec![240])).await?;
assert!(timeout(decoder.next()).await.is_err());
tx.send(Ok(vec![159])).await?;
assert!(timeout(decoder.next()).await.is_err());
tx.send(Ok(vec![146])).await?;
assert!(timeout(decoder.next()).await.is_err());
tx.send(Ok(vec![150])).await?;
assert_eq!("💖", timeout(decoder.next()).await?.unwrap()?);
assert!(timeout(decoder.next()).await.is_err());
```

# License

The code follows MIT license written in [LICENSE](./LICENSE). Contributors need
to agree that any modifications sent in this repository follow the license.
