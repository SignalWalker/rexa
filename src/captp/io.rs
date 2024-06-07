use std::{
    future::Future,
    num::NonZeroUsize,
    pin::Pin,
    task::{Context, Poll},
};

use syrup::{
    de::{Cursor, DecodeBytesError, DecodeError, LexError, LexErrorKind},
    Decode, TokenTree,
};

//pub struct BufferedSyrup<'reader, Reader: CapTpReadExt, Data> {
//    reader: &'reader mut Reader,
//    data: Data,
//    read_amt: usize,
//}
//
//impl<'reader, Reader: CapTpReadExt, Data> Deref for BufferedSyrup<'reader, Reader, Data> {
//    type Target = Data;
//
//    fn deref(&self) -> &Self::Target {
//        &self.data
//    }
//}
//
//impl<'reader, Reader: CapTpReadExt, Data> DerefMut for BufferedSyrup<'reader, Reader, Data> {
//    fn deref_mut(&mut self) -> &mut Self::Target {
//        &mut self.data
//    }
//}
//
//impl<'r, Reader: CapTpReadExt, Data> Drop for BufferedSyrup<'r, Reader, Data> {
//    fn drop(&mut self) {
//        self.reader.consume(self.read_amt)
//    }
//}

pub trait CapTpRead {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>>;
    fn consume(self: Pin<&mut Self>, amt: usize);
}

#[derive(thiserror::Error, Debug)]
pub enum ReadSyrupError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Lex(#[from] LexError),
    //Decode(DecodeBytesError<'input>),
}

pub trait CapTpReadExt: CapTpRead {
    /// Get the contents of the internal buffer, filling it from the internal reader if it's empty.
    ///
    /// Analogous to [`std::io::BufRead::fill_buf`].
    fn fill_buf<'result>(
        &'result mut self,
    ) -> impl Future<Output = std::io::Result<&'result [u8]>> + Send;
    /// Tell this buffer that `amt` bytes have been consumed and should no longer be returned by calls to [`fill_buf`](CapTpReadExt::fill_buf).
    ///
    /// Analogous to [`std::io::BufRead::consume`].
    fn consume(&mut self, amt: usize);

    /// Read some data and tokenize it into a [`TokenTree`], returning the tree and the amount of
    /// bytes it occupies, if successful. Read bytes are not consumed, so you should remember to consume
    /// them if you want to tokenize the rest of the data.
    fn try_read_syrup<'result>(
        &'result mut self,
    ) -> impl Future<Output = Result<(TokenTree<'result>, usize), ReadSyrupError>> + Send
    where
        Self: Send,
    {
        async move {
            let buf = self.fill_buf().await?;
            let (res, rem) = TokenTree::tokenize(Cursor::new(buf)).map_err(ReadSyrupError::Lex)?;
            Ok((res, buf.len() - rem.rem.len()))
        }
    }

    //fn read_syrup<'result>(
    //    &'result mut self,
    //) -> impl Future<Output = Result<(TokenTree<'result>, usize), ReadSyrupError>> {
    //    async move {
    //        loop {
    //            match self.try_read_syrup().await {
    //                Ok(res) => return Ok(res),
    //                Err(ReadSyrupError::Lex(LexError {
    //                    kind: LexErrorKind::Incomplete { .. },
    //                    ..
    //                })) => continue,
    //                Err(e) => return Err(e.into()),
    //            }
    //        }
    //    }
    //}

    /// Read some data and tokenize it into an owned [`TokenTree`]. If successful, the data is
    /// consumed from the buffer.
    fn try_consume_syrup(
        &mut self,
    ) -> impl Future<Output = Result<TokenTree<'static>, ReadSyrupError>> + Send
    where
        Self: Send,
    {
        async move {
            let input = Cursor::new(self.fill_buf().await?);
            let buf_len = input.rem.len();
            match TokenTree::tokenize_static(input) {
                Ok((res, rem)) => {
                    let amt_read = buf_len - rem.rem.len();
                    self.consume(amt_read);
                    Ok(res)
                }
                Err(e) => Err(e.into()),
            }
        }
    }

    fn consume_syrup(
        &mut self,
    ) -> impl Future<Output = Result<TokenTree<'static>, ReadSyrupError>> + Send
    where
        Self: Send,
    {
        async move {
            loop {
                match self.try_consume_syrup().await {
                    Ok(tree) => return Ok(tree),
                    Err(ReadSyrupError::Lex(LexError {
                        kind: LexErrorKind::Incomplete { .. },
                        ..
                    })) => continue,
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
}

pub struct FillBuf<'reader, Reader: ?Sized> {
    reader: Option<&'reader mut Reader>,
}

impl<'reader, Reader: CapTpRead + ?Sized + Unpin> Future for FillBuf<'reader, Reader> {
    type Output = std::io::Result<&'reader [u8]>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // this is pretty much verbatim from https://docs.rs/futures-util/0.3.30/src/futures_util/io/fill_buf.rs.html#29
        let this = &mut *self;
        let reader = this.reader.take().expect("Polled FillBuf after completion");

        match Pin::new(&mut *reader).poll_fill_buf(cx) {
            Poll::Ready(Ok(slice)) => {
                #[allow(unsafe_code)]
                // reason = laundering mutable reference that the compiler can't tell we're no longer using
                let slice: &'reader [u8] =
                    unsafe { std::slice::from_raw_parts(slice.as_ptr(), slice.len()) };
                Poll::Ready(Ok(slice))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => {
                this.reader = Some(reader);
                Poll::Pending
            }
        }
    }
}

impl<Reader: CapTpRead + Send + Unpin + ?Sized> CapTpReadExt for Reader {
    fn fill_buf<'s>(&'s mut self) -> FillBuf<'s, Self> {
        FillBuf { reader: Some(self) }
    }

    fn consume(&mut self, amt: usize) {
        Pin::new(self).consume(amt)
    }
}

#[cfg(feature = "tokio")]
impl<Reader: tokio::io::AsyncBufRead + ?Sized> CapTpRead for Reader {
    #[inline]
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        tokio::io::AsyncBufRead::poll_fill_buf(self, cx)
    }

    #[inline]
    fn consume(self: Pin<&mut Self>, amt: usize) {
        tokio::io::AsyncBufRead::consume(self, amt)
    }
}

#[cfg(not(feature = "tokio"))]
impl<Reader: futures::AsyncBufRead + ?Sized> CapTpRead for Reader {
    #[inline]
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        futures::AsyncBufRead::poll_fill_buf(self, cx)
    }

    #[inline]
    fn consume(self: Pin<&mut Self>, amt: usize) {
        futures::AsyncBufRead::consume(self, amt)
    }
}
