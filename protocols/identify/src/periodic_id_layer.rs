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

use futures::prelude::*;
use libp2p_core::nodes::{ConnectedPoint, NetworkBehaviour, NetworkBehaviourAction};
use libp2p_core::{protocols_handler::ProtocolsHandler, Multiaddr, PeerId};
use std::{collections::VecDeque, marker::PhantomData};
use tokio_io::{AsyncRead, AsyncWrite};
use {IdentifyInfo, PeriodicIdentification, PeriodicIdentificationEvent};

/// Network behaviour that automatically identifies nodes periodically, and returns information
/// about them.
pub struct PeriodicIdentifyBehaviour<TSubstream> {
    /// Events that need to be produced outside when polling..
    events: VecDeque<PeriodicIdentifyBehaviourEvent>,
    /// Marker to pin the generics.
    marker: PhantomData<TSubstream>,
}

impl<TSubstream> PeriodicIdentifyBehaviour<TSubstream> {
    /// Creates a `PeriodicIdentifyBehaviour`.
    pub fn new() -> Self {
        PeriodicIdentifyBehaviour {
            events: VecDeque::new(),
            marker: PhantomData,
        }
    }
}

impl<TSubstream> NetworkBehaviour for PeriodicIdentifyBehaviour<TSubstream>
where
    TSubstream: AsyncRead + AsyncWrite,
{
    type ProtocolsHandler = PeriodicIdentification<TSubstream>;
    type OutEvent = PeriodicIdentifyBehaviourEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        PeriodicIdentification::new()
    }

    fn inject_connected(&mut self, _: PeerId, _: ConnectedPoint) {}

    fn inject_disconnected(&mut self, _: &PeerId, _: ConnectedPoint) {}

    fn inject_node_event(
        &mut self,
        peer_id: PeerId,
        event: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent,
    ) {
        match event {
            PeriodicIdentificationEvent::Identified(remote) => {
                self.events
                    .push_back(PeriodicIdentifyBehaviourEvent::Identified {
                        peer_id: peer_id,
                        info: remote.info,
                        observed_addr: remote.observed_addr,
                    });
            }
            _ => (), // TODO: exhaustive pattern
        }
    }

    fn poll(
        &mut self,
    ) -> Async<
        NetworkBehaviourAction<
            <Self::ProtocolsHandler as ProtocolsHandler>::InEvent,
            Self::OutEvent,
        >,
    > {
        if let Some(event) = self.events.pop_front() {
            return Async::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Async::NotReady
    }
}

/// Event generated by the `PeriodicIdentifyBehaviour`.
#[derive(Debug, Clone)]
pub enum PeriodicIdentifyBehaviourEvent {
    /// We obtained identification information from the remote
    Identified {
        /// Peer that has been successfully identified.
        peer_id: PeerId,
        /// Information of the remote.
        info: IdentifyInfo,
        /// Address the remote observes us as.
        observed_addr: Multiaddr,
    },
}
