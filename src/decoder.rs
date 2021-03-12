use crate::error::DecodeError;
use futures_core::{ready, Stream};
use futures_io::AsyncRead;
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

pub type Result<T> = std::result::Result<T, DecodeError>;

pin_project! {
    pub struct Utf8Decoder<R> {
        #[pin]
        reader: R,
        buf: Box<[u8]>,
        remains: usize,
    }
}

impl<R> Utf8Decoder<R> {
    /// Create a new incremental UTF-8 decoder from `reader`
    pub fn new(reader: R) -> Self {
        Utf8Decoder::with_capacity(DEFAULT_BUF_SIZE, reader)
    }

    /// Create a new incremental UTF-8 decoder from `reader` with specified capacity
    pub fn with_capacity(capacity: usize, reader: R) -> Self {
        unsafe {
            let mut buffer = Vec::with_capacity(capacity);
            buffer.set_len(capacity);
            Self {
                reader,
                buf: buffer.into_boxed_slice(),
                remains: 0,
            }
        }
    }

    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.reader
    }

    /// Acquires a reference to the underlying reader that this
    /// decoder is pulling from.
    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    /// Acquires a mutable reference to the underlying reader that
    /// this decoder is pulling from.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }
}

impl<R> Stream for Utf8Decoder<R>
where
    R: AsyncRead + Unpin,
{
    type Item = Result<String>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<<Self as Stream>::Item>> {
        let this = self.project();
        let reader = this.reader;
        let buf = this.buf;
        let (decoded, remains) = ready!(decode_next(reader, cx, buf, *this.remains))?;
        *this.remains = remains;
        if decoded.is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(Some(Ok(decoded)))
        }
    }
}

fn decode_next<'a, R>(
    reader: Pin<&mut R>,
    cx: &mut Context<'_>,
    buf: &'a mut [u8],
    s: usize,
) -> Poll<Result<(String, usize)>>
where
    R: AsyncRead,
{
    debug_assert!(buf.len() > s);
    let n = ready!(reader.poll_read(cx, &mut buf[s..]))?;
    let e = s + n; //
    debug_assert!(buf.len() > e);
    let result = match std::str::from_utf8(&buf[..e]) {
        Ok(decoded) => Ok((decoded.to_string(), 0)),
        Err(err) => match err.error_len() {
            Some(_) => {
                // An unexpected byte was encounted. While this decoder is not
                // lossy decoding, return the error itself and stop decoding.
                Err(err.into())
            }
            None => {
                // The end of the input was reached unexpectedly. This is what
                // this decoder exists for.
                let (valid, after_valid) = buf.split_at(err.valid_up_to());
                // Copy 'valid' into the Heap as String
                let decoded = unsafe { std::str::from_utf8_unchecked(valid) };
                let decoded = decoded.to_string();
                // Copy 'after_valid' at the front of the 'buf'
                let remains = e - valid.len();
                unsafe {
                    // +-------------------------------------------------------------+
                    // |                            buf                              |
                    // +----------------+--------------------------------------------+
                    // |     valid      | after_valid                                |
                    // +----------------+--------------------------------------------+
                    // |////////////////|#####.......................................|
                    // +----------------+--------------------------------------------+
                    //                               |
                    //                               v
                    // +-------------------------------------------------------------+
                    // |                            buf                              |
                    // +----------------+--------------------------------------------+
                    // |     valid      | after_valid                                |
                    // +----------------+--------------------------------------------+
                    // |#####...........|............................                |
                    // +----------------+--------------------------------------------+
                    //
                    // XXX: Can we use 'copy_nonoverlapping' here?
                    // std::ptr::copy_nonoverlapping(after_valid.as_ptr(), buf.as_mut_ptr(), remains);
                    std::ptr::copy(after_valid.as_ptr(), buf.as_mut_ptr(), remains);
                }
                Ok((decoded, remains))
            }
        },
    };
    Poll::Ready(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use futures::channel::mpsc;
    use futures::io;
    use futures::prelude::*;
    use futures_await_test::async_test;

    async fn timeout<T>(future: impl Future<Output = T> + Unpin) -> Result<T> {
        let mut future = future.fuse();
        let mut sleep = futures_timer::Delay::new(std::time::Duration::from_millis(100)).fuse();
        futures::select! {
            r = future => Ok(r),
            _ = sleep => Err(anyhow!("Timeout")),
        }
    }

    #[async_test]
    async fn decoder_decode_demo() -> Result<()> {
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

        Ok(())
    }

    #[async_test]
    async fn decoder_decode_1byte_character() -> Result<()> {
        let (mut tx, rx) = mpsc::unbounded::<io::Result<Vec<u8>>>();
        let mut decoder = Utf8Decoder::new(rx.into_async_read());

        tx.send(Ok(vec![0x24])).await?;
        assert_eq!("\u{0024}", timeout(decoder.next()).await?.unwrap()?);
        assert!(timeout(decoder.next()).await.is_err());

        Ok(())
    }

    #[async_test]
    async fn decoder_decode_2byte_character() -> Result<()> {
        let (mut tx, rx) = mpsc::unbounded::<io::Result<Vec<u8>>>();
        let mut decoder = Utf8Decoder::new(rx.into_async_read());

        // Complete
        tx.send(Ok(vec![0xC2, 0xA2])).await?;
        assert_eq!("\u{00A2}", timeout(decoder.next()).await?.unwrap()?);
        assert!(timeout(decoder.next()).await.is_err());

        // Incremental
        tx.send(Ok(vec![0xC2])).await?;
        assert!(timeout(decoder.next()).await.is_err());
        tx.send(Ok(vec![0xA2])).await?;
        assert_eq!("\u{00A2}", timeout(decoder.next()).await?.unwrap()?);
        assert!(timeout(decoder.next()).await.is_err());

        Ok(())
    }

    #[async_test]
    async fn decoder_decode_3byte_character() -> Result<()> {
        let (mut tx, rx) = mpsc::unbounded::<io::Result<Vec<u8>>>();
        let mut decoder = Utf8Decoder::new(rx.into_async_read());

        // Complete
        tx.send(Ok(vec![0xE0, 0xA4, 0xB9])).await?;
        assert_eq!("\u{0939}", timeout(decoder.next()).await?.unwrap()?);
        assert!(timeout(decoder.next()).await.is_err());

        // Incremental
        tx.send(Ok(vec![0xE0])).await?;
        assert!(timeout(decoder.next()).await.is_err());
        tx.send(Ok(vec![0xA4])).await?;
        assert!(timeout(decoder.next()).await.is_err());
        tx.send(Ok(vec![0xB9])).await?;
        assert_eq!("\u{0939}", timeout(decoder.next()).await?.unwrap()?);
        assert!(timeout(decoder.next()).await.is_err());

        Ok(())
    }

    #[async_test]
    async fn decoder_decode_4byte_character() -> Result<()> {
        let (mut tx, rx) = mpsc::unbounded::<io::Result<Vec<u8>>>();
        let mut decoder = Utf8Decoder::new(rx.into_async_read());

        // Complete
        tx.send(Ok(vec![0xF0, 0x90, 0x8D, 0x88])).await?;
        assert_eq!("\u{10348}", timeout(decoder.next()).await?.unwrap()?);
        assert!(timeout(decoder.next()).await.is_err());

        // Incremental
        tx.send(Ok(vec![0xF0])).await?;
        assert!(timeout(decoder.next()).await.is_err());
        tx.send(Ok(vec![0x90])).await?;
        assert!(timeout(decoder.next()).await.is_err());
        tx.send(Ok(vec![0x8D])).await?;
        assert!(timeout(decoder.next()).await.is_err());
        tx.send(Ok(vec![0x88])).await?;
        assert_eq!("\u{10348}", timeout(decoder.next()).await?.unwrap()?);
        assert!(timeout(decoder.next()).await.is_err());

        Ok(())
    }

    #[async_test]
    async fn decoder_decode_ok() -> Result<()> {
        let (mut tx, rx) = mpsc::unbounded::<io::Result<Vec<u8>>>();
        let mut decoder = Utf8Decoder::new(rx.into_async_read());

        tx.send(Ok(vec![
            0x24, 0xC2, 0xA2, 0xE0, 0xA4, 0xB9, 0xF0, 0x90, 0x8D, 0x88,
        ]))
        .await?;
        tx.send(Ok(vec![
            0x24, 0xC2, 0xA2, 0xE0, 0xA4, 0xB9, 0xF0, 0x90, 0x8D, 0x88,
        ]))
        .await?;
        tx.send(Ok(vec![
            0x24, 0xC2, 0xA2, 0xE0, 0xA4, 0xB9, 0xF0, 0x90, 0x8D, 0x88,
        ]))
        .await?;
        assert_eq!(
            "\u{0024}\u{00A2}\u{0939}\u{10348}",
            timeout(decoder.next()).await?.unwrap()?
        );
        assert_eq!(
            "\u{0024}\u{00A2}\u{0939}\u{10348}",
            timeout(decoder.next()).await?.unwrap()?
        );
        assert_eq!(
            "\u{0024}\u{00A2}\u{0939}\u{10348}",
            timeout(decoder.next()).await?.unwrap()?
        );
        assert!(timeout(decoder.next()).await.is_err());

        Ok(())
    }
}
