// Copyright 2018 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

extern crate bytes;
extern crate futures;
#[macro_use]
extern crate log;
extern crate libp2p_core;
extern crate tokio_io;
extern crate yamux;
#[cfg(test)]
extern crate libp2p_test_muxer;

use bytes::Bytes;
use futures::{future::{self, FutureResult}, prelude::*};
use libp2p_core::{muxing::Shutdown, upgrade::{InboundUpgrade, OutboundUpgrade, UpgradeInfo}};
use std::{io, iter};
use std::io::{Error as IoError};
use tokio_io::{AsyncRead, AsyncWrite};
use std::fmt;

#[derive(Debug)]
pub struct Yamux<C>(yamux::Connection<C>);

impl<C> Yamux<C>
where
    C: AsyncRead + AsyncWrite + 'static
{
    pub fn new(c: C, cfg: yamux::Config, mode: yamux::Mode) -> Self {
        Yamux(yamux::Connection::new(c, cfg, mode))
    }
}

impl<C> libp2p_core::StreamMuxer for Yamux<C>
where
    C: AsyncRead + AsyncWrite + 'static
{
    type Substream = yamux::StreamHandle<C>;
    type OutboundSubstream = FutureResult<Option<Self::Substream>, io::Error>;

    #[inline]
    fn poll_inbound(&self) -> Poll<Option<Self::Substream>, IoError> {
        match self.0.poll() {
            Err(e) => {
                error!("connection error: {}", e);
                Err(io::Error::new(io::ErrorKind::Other, e))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(stream))) => Ok(Async::Ready(Some(stream)))
        }
    }

    #[inline]
    fn open_outbound(&self) -> Self::OutboundSubstream {
        let stream = self.0.open_stream().map_err(|e| io::Error::new(io::ErrorKind::Other, e));
        future::result(stream)
    }

    #[inline]
    fn poll_outbound(&self, substream: &mut Self::OutboundSubstream) -> Poll<Option<Self::Substream>, IoError> {
        substream.poll()
    }

    #[inline]
    fn destroy_outbound(&self, _: Self::OutboundSubstream) {
    }

    #[inline]
    fn read_substream(&self, sub: &mut Self::Substream, buf: &mut [u8]) -> Poll<usize, IoError> {
        sub.poll_read(buf)
    }

    #[inline]
    fn write_substream(&self, sub: &mut Self::Substream, buf: &[u8]) -> Poll<usize, IoError> {
        sub.poll_write(buf)
    }

    #[inline]
    fn flush_substream(&self, sub: &mut Self::Substream) -> Poll<(), IoError> {
        sub.poll_flush()
    }

    #[inline]
    fn shutdown_substream(&self, sub: &mut Self::Substream, _: Shutdown) -> Poll<(), IoError> {
        sub.shutdown()
    }

    #[inline]
    fn destroy_substream(&self, _: Self::Substream) {
    }

    #[inline]
    fn shutdown(&self, _: Shutdown) -> Poll<(), IoError> {
        self.0.shutdown()
    }

    #[inline]
    fn flush_all(&self) -> Poll<(), IoError> {
        self.0.flush()
    }
}

#[derive(Clone)]
pub struct Config(yamux::Config);

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.0)
    }
}

impl Config {
    pub fn new(cfg: yamux::Config) -> Self {
        Config(cfg)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config(yamux::Config::default())
    }
}

impl UpgradeInfo for Config {
    type UpgradeId = ();
    type NamesIter = iter::Once<(Bytes, Self::UpgradeId)>;

    fn protocol_names(&self) -> Self::NamesIter {
        iter::once((Bytes::from("/yamux/1.0.0"), ()))
    }
}

impl<C> InboundUpgrade<C> for Config
where
    C: AsyncRead + AsyncWrite + 'static,
{
    type Output = Yamux<C>;
    type Error = io::Error;
    type Future = FutureResult<Yamux<C>, io::Error>;

    fn upgrade_inbound(self, i: C, _: Self::UpgradeId) -> Self::Future {
        future::ok(Yamux::new(i, self.0, yamux::Mode::Server))
    }
}

impl<C> OutboundUpgrade<C> for Config
where
    C: AsyncRead + AsyncWrite + 'static,
{
    type Output = Yamux<C>;
    type Error = io::Error;
    type Future = FutureResult<Yamux<C>, io::Error>;

    fn upgrade_outbound(self, i: C, _: Self::UpgradeId) -> Self::Future {
        future::ok(Yamux::new(i, self.0, yamux::Mode::Client))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use libp2p_test_muxer::test_muxer;

    #[test]
    fn test_muxing() {
        test_muxer(Config::default())
    }
}
