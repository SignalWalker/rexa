use crate::async_compat::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Mutex};
use syrup::{Deserialize, Serialize};

#[derive(Debug)]
pub(super) struct CapTpSessionCore<Reader, Writer> {
    pub(super) reader: Mutex<Reader>,
    pub(super) writer: Mutex<Writer>,
}

impl<Reader, Writer> CapTpSessionCore<Reader, Writer> {
    pub(super) fn new(reader: Reader, writer: Writer) -> Self {
        Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
        }
    }

    #[inline]
    pub(super) async fn recv(&self, buf: &mut [u8]) -> Result<usize, std::io::Error>
    where
        Reader: AsyncRead + Unpin,
    {
        self.reader.lock().await.read(buf).await
    }

    // #[inline]
    // pub(super) fn send<'write>(
    //     &'write mut self,
    //     buf: &'write [u8],
    // ) -> impl Future<Output = Result<usize, futures::io::Error>> + 'write
    // where
    //     Socket: AsyncWrite + Unpin,
    // {
    //     self.socket.write(buf)
    // }

    #[inline]
    pub(super) async fn send_all(&self, buf: &[u8]) -> Result<(), std::io::Error>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.writer.lock().await.write_all(buf).await
    }

    pub(super) async fn send_msg<Msg: Serialize>(&self, msg: &Msg) -> Result<(), std::io::Error>
    where
        Writer: AsyncWrite + Unpin,
    {
        // TODO :: custom error type
        self.send_all(&syrup::ser::to_bytes(msg).unwrap()).await
    }

    pub(super) async fn flush(&self) -> Result<(), std::io::Error>
    where
        Writer: AsyncWrite + Unpin,
    {
        self.writer.lock().await.flush().await
    }

    pub(super) async fn recv_msg<'de, Msg: Deserialize<'de>>(
        &self,
        recv_buf: &'de mut [u8],
    ) -> Result<Msg, std::io::Error>
    where
        Reader: AsyncRead + Unpin,
    {
        let amt = self.recv(recv_buf).await?;
        Ok(syrup::de::from_bytes(&recv_buf[..amt]).unwrap())
    }
}
