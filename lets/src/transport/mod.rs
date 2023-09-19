// Rust
use alloc::{boxed::Box, rc::Rc, vec::Vec};
use core::cell::RefCell;

// 3rd-party
use async_trait::async_trait;

// IOTA

// Streams

// Local
use crate::{
    address::Address,
    error::{Error, Result},
    message::TransportMessage,
};

/// Network transport abstraction.
/// Parametrized by the type of message addresss.
/// Message address is used to identify/locate a message (eg. like URL for HTTP).
#[async_trait(?Send)]
pub trait Transport<'a> {
    type Msg;
    type SendResponse;
    /// Send a message
    async fn send_message(&mut self, address: Address, msg: Self::Msg) -> Result<Self::SendResponse>
    where
        'a: 'async_trait;

    /// Receive messages
    async fn recv_messages(&mut self, address: Address) -> Result<Vec<Self::Msg>>
    where
        'a: 'async_trait;

    /// Receive a single message
    async fn recv_message(&mut self, address: Address) -> Result<Self::Msg> {
        let mut msgs = self.recv_messages(address).await?;
        if let Some(msg) = msgs.pop() {
            match msgs.is_empty() {
                true => Ok(msg),
                // TODO - CGE: AddressError should be split into errors AddressNotFound and FoundMultipleMessages
                //             Currently only the comment string can be used to distinguish between both cases
                false => Err(Error::AddressError("More than one found", address)),
            }
        } else {
            Err(Error::AddressError("not found in transport", address))
        }
    }
}

#[async_trait(?Send)]
impl<'a, Tsp: Transport<'a>> Transport<'a> for Rc<RefCell<Tsp>> {
    type Msg = Tsp::Msg;
    type SendResponse = Tsp::SendResponse;

    /// Send a message.
    async fn send_message(&mut self, address: Address, msg: Tsp::Msg) -> Result<Tsp::SendResponse>
    where
        Self::Msg: 'async_trait,
    {
        self.borrow_mut().send_message(address, msg).await
    }

    /// Receive messages with default options.
    async fn recv_messages(&mut self, address: Address) -> Result<Vec<Tsp::Msg>> {
        self.borrow_mut().recv_messages(address).await
    }
}

/// Interface for message indexing services.
/// Implementations of the Transport trait need to index all transported messages by Address
/// msg_index values (return value of the Address::to_msg_index() function).
/// For transport media that don't allow indexes for custom message tags or custom ids,
/// (example: IOTA tangle), an additional indexing service is needed.
#[async_trait(?Send)]
pub trait MessageIndex<Message = TransportMessage> {
    /// Return all messages that match the specified msg_index.
    async fn get_messages_by_msg_index(&self, msg_index: [u8; 32]) -> Result<Vec<Message>>;
    /// Indexing services may need to modify the msg_index, example given for filtering
    /// purposes. The get_tag_value() function should be used by transport Client implementations
    /// to fetch the final value that is used to tag the message before it is send via the transport
    /// medium.
    fn get_tag_value(&self, msg_index: [u8; 32]) -> Result<Vec<u8>>;
}

/// Localised mapping for tests and simulations
pub mod bucket;
/// `iota.rs` based tangle client
#[cfg(any(feature = "tangle-client", feature = "tangle-client-wasm"))]
pub mod tangle;
/// Localised micro tangle client
#[cfg(feature = "utangle-client")]
pub mod utangle;
