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

extern crate libp2p_core;
extern crate tokio;
extern crate tokio_io;
extern crate bytes;
extern crate futures;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate libp2p_tcp_transport as tcp;

use tcp::{TcpConfig, TcpTransStream};

use libp2p_core::{
    Transport,
    StreamMuxer,
    muxing::{self, SubstreamRef},
    upgrade::{InboundUpgrade, OutboundUpgrade, UpgradeInfo},
    transport::upgrade::ListenerUpgradeFuture,
};

use tokio::runtime::Runtime;
use tokio_io::codec::length_delimited::Framed;
use futures::prelude::*;
use std::thread;
use std::sync::mpsc;
use std::fmt::Debug;
use std::sync::Arc;

pub fn test_muxer<U, O, E>(config: U)
where
    U: OutboundUpgrade<TcpTransStream, Output = O, Error = E> + Send + Clone + 'static,
    U: InboundUpgrade<TcpTransStream, Output = O, Error = E>,
    U: Debug, // needed for `unwrap()`
    <U as UpgradeInfo>::NamesIter: Send,
    <U as UpgradeInfo>::UpgradeId: Send,
    <U as InboundUpgrade<TcpTransStream>>::Future: Send,
    <U as OutboundUpgrade<TcpTransStream>>::Future: Send,
    E: std::error::Error + Send + Sync + 'static,
    O: StreamMuxer + Send + Sync + 'static  + std::fmt::Debug,
    <O as StreamMuxer>::Substream: Send + Sync + std::fmt::Debug,
    <O as StreamMuxer>::OutboundSubstream: Send + Sync,
{
    env_logger::init();
//    client_to_server_outbound(config.clone());
    client_to_server_inbound(config.clone());
}
// at this point I need to port over the other test as well, and with that I'd have *something*
// next step is to build a few helpers to build up the futures and start adding asserts

fn client_to_server_inbound<U, O, E>(config: U)
    where
        U: OutboundUpgrade<TcpTransStream, Output = O, Error = E> + Send + Clone + 'static,
        U: InboundUpgrade<TcpTransStream, Output = O, Error = E>,
        U: Debug, // needed for `unwrap()`
        <U as UpgradeInfo>::NamesIter: Send,
        <U as UpgradeInfo>::UpgradeId: Send,
        <U as InboundUpgrade<TcpTransStream>>::Future: Send,
        <U as OutboundUpgrade<TcpTransStream>>::Future: Send,
        E: std::error::Error + Send + Sync + 'static,
        O: StreamMuxer + Send + Sync + 'static + std::fmt::Debug,
        <O as StreamMuxer>::Substream: Send + Sync + std::fmt::Debug,
        <O as StreamMuxer>::OutboundSubstream: Send + Sync,
{
    let (tx, rx) = mpsc::channel();
    let config2 = config.clone();
    let bg_thread = thread::spawn(move || {
        trace!("[thread] START");
        let transport = TcpConfig::new().with_upgrade(config2);
        let (listener, addr) = transport
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        tx.send(addr).expect("sending the address across threads works");
        let future = listener
            .into_future()
            .map_err(|(e, _)| e)
            .and_then(|(client, _)| client.expect("bbb").0 )
            .and_then(|client| {
//            .and_then(|client: ListenerUpgradeFuture<_, U>| {
                // TODO: this returns None – how?
                trace!("[thread] client={:?}", client);
//                muxing::inbound_from_ref_and_wrap(Arc::new(client))
                muxing::inbound_from_ref_and_wrap(Arc::new(client))
            })
//            .map(|client| Framed::<SubstreamRef<_>>::new(client.expect("ccc")))
            .map(|client| {
                if client.is_none() {
                    trace!("[thread] client is None");
                    panic!("[thread] oh noes!");
                } else {
                    trace!("[thread] client is Some");
                    Framed::<_, bytes::BytesMut>::new(client.expect("ccc"))
                }
            })
            .and_then(|client| client
                .into_future()
                .map_err(|(e, _)| e)
                .map(|(msg, _)| msg)
            )
            .and_then(|msg| {
                trace!("[thread] got a message={:?}", msg);
                let msg = msg.expect("gets a message");
//                assert_eq!(msg, "hello world");
                Ok(())
            });
        let mut rt = Runtime::new().expect("toko works 1");
        let _ = rt.block_on(future).expect("block on works");
    });

    let transport = TcpConfig::new().with_upgrade(config);
    let fut = transport
//        .dial(addr).expect("dial to work")
        .dial(rx.recv().unwrap()).expect("dial to work")
        .and_then(|client| {
            trace!("[dial] first and_then");
            muxing::outbound_from_ref_and_wrap(Arc::new(client))
        })
        .map(|server| {
            trace!("[dial] setting up Framed");
            Framed::<SubstreamRef<_>>::new(server.expect("substreamref"))
        })
        .and_then(|server| {
            trace!("[dial] sending");
            server.send("hello world".into())
        })
        .map(|_| ());

    let mut rt = Runtime::new().expect("tokio to work");
    let _ = rt.block_on(fut).expect("run a future");
    bg_thread.join().expect("joining a thread works");
}

fn client_to_server_outbound<U, O, E>(config: U)
    where
        U: OutboundUpgrade<TcpTransStream, Output = O, Error = E> + Send + Clone + 'static,
        U: InboundUpgrade<TcpTransStream, Output = O, Error = E>,
        U: Debug, // needed for `unwrap()`
        <U as UpgradeInfo>::NamesIter: Send,
        <U as UpgradeInfo>::UpgradeId: Send,
        <U as InboundUpgrade<TcpTransStream>>::Future: Send,
        <U as OutboundUpgrade<TcpTransStream>>::Future: Send,
        E: std::error::Error + Send + Sync + 'static,
        O: StreamMuxer + Send + Sync + 'static + std::fmt::Debug,
        <O as StreamMuxer>::Substream: Send + Sync + std::fmt::Debug,
        <O as StreamMuxer>::OutboundSubstream: Send + Sync,
{
    let (tx, rx) = mpsc::channel();
    let listener_config = config.clone();
    let thr = thread::spawn(move || {
        let transport = TcpConfig::new().with_upgrade(listener_config);
        let (listener, addr) = transport
            .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
        tx.send(addr).unwrap();

        let mut rt_listener = Runtime::new().unwrap();
        let fut = listener
            // convert stream to future yielding a tuple ("next stream item", "rest of stream")
            .into_future()
            // convert the error type from `(err, ?)` to `err`
            .map_err(|(e, _)| e)
            // maybe_muxer is an `Option<(Future<Item=O, …>, Multiaddr)>` (the ignored
            // tuple item is the rest of the Stream)
            .and_then(|(maybe_muxer, _)| maybe_muxer.unwrap().0)
            .and_then(|muxer: O| {
                // This calls `open_outbound()` on the `StreamMuxer` and returns
                // an `OutboundSubstreamRefWrapFuture<Arc<…>>`.
                // take the muxer and build a future out of it, that, when resolved, yields
                // a substream we can set up the framed codec on.
                muxing::outbound_from_ref_and_wrap(Arc::new(muxer))
            })
            .map(|substream: Option<SubstreamRef<Arc<O>>>| Framed::<_, bytes::BytesMut>::new(substream.unwrap()))
            // substream is a `Framed<SubstreamRef<Arc<O>>>`, and `Framed` is a `Stream`
            .and_then(|substream: Framed<SubstreamRef<Arc<O>>>| {
                substream.into_future()
                    .map_err(|(e, _): (std::io::Error, Framed<SubstreamRef<Arc<O>>>)| e)
                    .map(|(msg, _)| msg)
            })
            .and_then(|msg| {
                trace!("message received: {:?}", msg);
                Ok(())
            });
        rt_listener.block_on(fut).unwrap();
    });

    let addr = rx.recv().unwrap();
    info!("Listening on {:?}", addr);

    let transport = TcpConfig::new().with_upgrade(config);
    let fut = transport.dial(addr).unwrap()
        .and_then(|muxer: O| {
            muxing::inbound_from_ref_and_wrap(Arc::new(muxer))
        })
        .map(|server: Option<SubstreamRef<Arc<O>>>| Framed::<_, bytes::BytesMut>::new(server.unwrap()))
        .and_then(|server: Framed<SubstreamRef<Arc<O>>>| {
            // TODO: why is the `server` consumed after sending the message? Should I be using `send_all` and drive the Future forward instead?
            server.send("hello".into())
        });

    let mut rt_dialer = Runtime::new().unwrap();
    let _ = rt_dialer.block_on(fut).unwrap();
    thr.join().unwrap();
}
