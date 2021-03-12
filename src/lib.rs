//! # Asynchronous and incremental UTF-8 decoder
//!
//! `async-utf8-decoder` crate provides `Utf8Decoder` which allows to convert any object which
//! implements `AsyncRead` trait into a string stream which implements `Stream` trait.
//!
//! ## Example
//!
//! ```
//! # use anyhow::{anyhow, Result};
//! # use futures::prelude::*;
//! # use futures::executor;
//! # async fn timeout<T>(future: impl Future<Output = T> + Unpin) -> Result<T> {
//! #     let mut future = future.fuse();
//! #     let mut sleep = futures_timer::Delay::new(std::time::Duration::from_millis(100)).fuse();
//! #     futures::select! {
//! #         r = future => Ok(r),
//! #         _ = sleep => Err(anyhow!("Timeout")),
//! #     }
//! # }
//! # fn main() -> Result<()> {
//! # executor::block_on(async {
//! use futures::io;
//! use futures::channel::mpsc;
//! use async_utf8_decoder::Utf8Decoder;
//!
//! let (mut tx, rx) = mpsc::unbounded::<io::Result<Vec<u8>>>();
//! let mut decoder = Utf8Decoder::new(rx.into_async_read());
//!
//! tx.send(Ok(vec![240])).await?;
//! assert!(timeout(decoder.next()).await.is_err());
//! tx.send(Ok(vec![159])).await?;
//! assert!(timeout(decoder.next()).await.is_err());
//! tx.send(Ok(vec![146])).await?;
//! assert!(timeout(decoder.next()).await.is_err());
//! tx.send(Ok(vec![150])).await?;
//! assert_eq!("💖", timeout(decoder.next()).await?.unwrap()?);
//! assert!(timeout(decoder.next()).await.is_err());
//! # Ok(()) as Result<()>
//! # })?;
//! # Ok(())
//! # }
//! ```
//!
pub mod decoder;
pub mod error;

#[doc(inline)]
pub use decoder::{Result, Utf8Decoder};
#[doc(inline)]
pub use error::DecodeError;
