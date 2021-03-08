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
    pub fn new(reader: R) -> Self {
        Utf8Decoder::with_capacity(DEFAULT_BUF_SIZE, reader)
    }

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

    pub fn into_inner(self) -> R {
        self.reader
    }

    pub fn get_ref(&self) -> &R {
        &self.reader
    }

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
        Poll::Ready(Some(Ok(decoded)))
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
    use anyhow::Result;
    use async_std::io::Cursor;
    use async_std::prelude::*;

    #[async_std::test]
    async fn decoder_ok() -> Result<()> {
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

        // Decode lot
        append(
            decoder.get_mut(),
            &vec![
                240, 159, 146, 150, 240, 159, 146, 150, 240, 159, 146, 150, 240, 159, 146, 150,
                240, 159, 146, 150,
            ],
        )
        .await?;
        let decoded = decoder.next().await.unwrap()?;
        assert_eq!("💖💖💖💖💖", decoded);

        Ok(())
    }
}
