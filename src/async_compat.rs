#[cfg(not(feature = "tokio"))]
pub(crate) use futures::{
    channel::{
        mpsc,
        oneshot::{self, Canceled as OneshotRecvError},
    },
    lock::Mutex,
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
};
#[cfg(feature = "tokio")]
pub(crate) use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{
        mpsc,
        oneshot::{self, error::RecvError as OneshotRecvError},
        Mutex, RwLock,
    },
};

// /// Copy of [futures::io::AsyncRead] so that we can implement it for [tokio::io::AsyncRead] when `#[cfg(feature = "tokio")]`.
// pub trait AsyncRead {
//     fn poll_read(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &mut [u8],
//     ) -> Poll<Result<usize, std::io::Error>>;
//
//     fn poll_read_vectored(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         bufs: &mut [IoSliceMut<'_>],
//     ) -> Poll<Result<usize, std::io::Error>>;
// }
//
// impl<F: futures::io::AsyncRead> AsyncRead for F {
//     fn poll_read(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &mut [u8],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         futures::io::AsyncRead::poll_read(self, cx, buf)
//     }
//
//     fn poll_read_vectored(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         bufs: &mut [IoSliceMut<'_>],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         todo!()
//     }
// }
//
// impl<T: tokio::io::AsyncRead> AsyncRead for T {
//     fn poll_read(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &mut [u8],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         todo!()
//     }
//
//     fn poll_read_vectored(
//         self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         bufs: &mut [IoSliceMut<'_>],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         todo!()
//     }
// }
