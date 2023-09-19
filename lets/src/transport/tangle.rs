// Rust
use alloc::{boxed::Box, vec::Vec};
use core::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

// 3rd-party
use async_trait::async_trait;

// IOTA
use iota_sdk::{
    client::{
        Client as IotaClient,
        builder::ClientBuilder
    },
    types::block::{
        Block,
        payload::Payload,
    }
};

// Streams

// Local
use crate::{
    address::Address,
    error::{Error, Result},
    message::TransportMessage,
    transport::{
        Transport,
        MessageIndex,
    },
};

/// A [`Transport`] Client for sending and retrieving binary messages from an `IOTA Tangle` node.
/// This Client uses the [iota.rs](https://github.com/iotaledger/iota.rs) Client implementation.
#[derive(Debug)]
pub struct Client<MsgIndex, Message = TransportMessage, SendResponse = TransportMessage> {
    iota_client: IotaClient,
    msg_index: MsgIndex,
    _phantom: PhantomData<(Message, SendResponse)>,
}

impl<MsgIndex, Message, SendResponse> Client<MsgIndex, Message, SendResponse> {
    /// Create an instance of [`Client`] with an  explicit client
    pub fn new(client: IotaClient, msg_index: MsgIndex) -> Self {
        Self{iota_client: client, msg_index, _phantom: PhantomData}
    }

    /// Shortcut to create an instance of [`Client`] connecting to a node with default parameters
    ///
    /// # Arguments
    /// * `node_url`: URL endpoint for node operations
    pub async fn for_node(node_url: &str, msg_index: MsgIndex) -> Result<Client<MsgIndex, Message, SendResponse>> {
        Ok(Self {
            iota_client: ClientBuilder::new()
                .with_node(node_url)
                .map_err( | e| Error::IotaClient("building client",
                e)) ?
                .with_local_pow(true)
                .finish()
                .await
                .map_err( | e| Error::External(e.into())) ?,
            msg_index,
            _phantom: PhantomData,
        })
    }

    /// Returns a reference to the `IOTA` [Client](`IotaClient`)
    pub fn client(&self) -> &IotaClient {
        &self.iota_client
    }

    /// Returns a mutable reference to the `IOTA` [Client](`IotaClient`)
    pub fn client_mut(&mut self) -> &mut IotaClient {
        &mut self.iota_client
    }
}

#[async_trait(?Send)]
impl<MsgIndex, Message, SendResponse> Transport<'_> for Client<MsgIndex, Message, SendResponse>
where
    Message: Into<Vec<u8>> + TryFrom<Block, Error = crate::error::Error>,
    SendResponse: TryFrom<Block, Error = crate::error::Error>,
    MsgIndex: MessageIndex<Message>
{
    type Msg = Message;
    type SendResponse = SendResponse;

    /// Sends a message indexed at the provided [`Address`] to the tangle.
    ///
    /// # Arguments
    /// * `address`: The address of the message to send.
    /// * `msg`: Message - The message to send.
    async fn send_message(&mut self, address: Address, msg: Message) -> Result<SendResponse>
    where
        Message: 'async_trait,
    {
        let tag = self.msg_index.get_tag_value(address.to_msg_index())?;
        self.iota_client
            .build_block()
            .with_tag(tag)
            .with_data(msg.into())
            .finish()
            .await
            .map_err(|e| Error::IotaClient("sending message", e))?
            .try_into()
    }

    /// Retrieves a message indexed at the provided [`Address`] from the tangle. Errors if no
    /// messages are found.
    ///
    /// # Arguments
    /// * `address`: The address of the message to retrieve.
    async fn recv_messages(&mut self, address: Address) -> Result<Vec<Message>> {
        let msgs = self
            .msg_index
            .get_messages_by_msg_index(address.to_msg_index())
            .await?;

        if msgs.is_empty() {
            return Err(Error::MessageMissing(address, "transport"));
        }

        Ok(msgs)
    }
}

impl TryFrom<Block> for TransportMessage {
    type Error = crate::error::Error;
    fn try_from(block: Block) -> Result<Self> {
        if let Some(Payload::TaggedData(tagged_data)) = block.payload() {
            Ok(Self::new(tagged_data.data().into()))
        } else {
            Err(Error::Malformed(
                "payload from the Tangle",
                "TaggedData",
                alloc::string::ToString::to_string(&block.id()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        address::{Address, AppAddr, MsgId},
        id::Identifier,
        message::{Topic, TransportMessage},
    };

    use super::*;

    #[tokio::test]
    async fn send_message() -> Result<()> {
        let mut client = Client::for_node("https://chrysalis-nodes.iota.org").await?;
        let msg = TransportMessage::new(vec![12; 1024]);
        let response: TransportMessage = client
            .send_message(
                Address::new(
                    AppAddr::default(),
                    MsgId::gen(
                        AppAddr::default(),
                        &Identifier::default(),
                        &Topic::default(),
                        Utc::now().timestamp_millis() as u32,
                    ),
                ),
                msg.clone(),
            )
            .await?;
        assert_eq!(msg, response);
        Ok(())
    }
}
