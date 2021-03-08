# rs-async-utf8-decoder

[![Audit](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/audit.yml/badge.svg)](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/audit.yml)
[![Build](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/build.yml/badge.svg)](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/build.yml)
[![Test](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/test.yml/badge.svg)](https://github.com/lambdalisue/rs-async-utf8-decoder/actions/workflows/test.yml)

Incremental UTF8 decoder which convert [`AsyncRead`][] into [`Stream`][] of [futures-rs][].

[`AsyncRead`]: https://docs.rs/futures/0.3.13/futures/prelude/trait.AsyncRead.html 
[`Stream`]: https://docs.rs/futures/0.3.13/futures/stream/trait.Stream.html
[futures-rs]: https://docs.rs/futures/0.3.13/futures/index.html 

## Example

```rust
use async_std::io::Cursor;
use async_utf8_decoder::Utf8Decoder;

let cur = Cursor::new(Vec::new());
let mut decoder = Utf8Decoder::new(cur);

async fn append(cursor: &mut Cursor<Vec<u8>>, data: &[u8]) -> Result<()> {
    let p = cursor.position();
    cursor.set_position(cursor.get_ref().len() as u64);
    cursor.write(data).await?;
    cursor.set_position(p);
    Ok(())
}

// Decode full
append(decoder.get_mut(), &vec![240, 159, 146, 150]).await?;
let decoded = decoder.next().await.unwrap()?;
assert_eq!("💖", decoded);

// Decode half
append(decoder.get_mut(), &vec![240, 159]).await?;
let decoded = decoder.next().await.unwrap()?;
assert_eq!("", decoded);
append(decoder.get_mut(), &vec![146, 150]).await?;
let decoded = decoder.next().await.unwrap()?;
assert_eq!("💖", decoded);

// Decode char
append(decoder.get_mut(), &vec![240]).await?;
let decoded = decoder.next().await.unwrap()?;
assert_eq!("", decoded);
append(decoder.get_mut(), &vec![159]).await?;
let decoded = decoder.next().await.unwrap()?;
assert_eq!("", decoded);
append(decoder.get_mut(), &vec![146]).await?;
let decoded = decoder.next().await.unwrap()?;
assert_eq!("", decoded);
append(decoder.get_mut(), &vec![150]).await?;
let decoded = decoder.next().await.unwrap()?;
assert_eq!("💖", decoded);
```

## License

The code follows MIT license written in [LICENSE](./LICENSE). Contributors need
to agree that any modifications sent in this repository follow the license.
