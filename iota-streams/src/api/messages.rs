// Rust
use alloc::{
    boxed::Box,
    collections::VecDeque,
    vec::Vec,
};
use core::{
    fmt::Display,
    future::Future,
    hash::Hash,
    pin::Pin,
};
use spongos::{
    ddml::commands::{
        unwrap,
        Absorb,
    },
    PRP,
};

// 3rd-party
use anyhow::Result;
use async_recursion::async_recursion;
use futures::{
    future,
    task::{
        Context,
        Poll,
    },
    Stream,
    StreamExt,
    TryStream,
    TryStreamExt,
};
use hashbrown::HashMap;

// IOTA

// Streams
use LETS::{
    id::Identifier,
    link::{
        Link,
        LinkGenerator,
    },
    message::{
        TransportMessage,
        HDF,
    },
    transport::Transport,
};

// Local
use crate::api::{
    message::{
        Message,
        MessageContent,
        Orphan,
    },
    user::User,
};

// TODO: aclarative comments
// TODO: Documentation
// TODO: backwards stream
// TODO: next_msg in C bindings
// TODO: Consider renaming msgs => messages
// TODO: run examples in actions

pub(crate) trait IntoMessages<T, F, A, AG> {
    fn messages(&mut self) -> Messages<'_, T, F, A, AG>
    where
        A: Link,
        A::Relative: Eq + Hash;
}

/// a [`Stream`] over the messages of the channel pending to be fetch from the transport
///
/// Use this stream to preorderly traverse the messages of the channel. This stream is usually
/// created from any type implementing [`IntoMessages`], calling its [`IntoMessages::messages()`] method.
/// The main method is [`Messages::next()`], which returns the next message in the channel that is readable
/// by the user.
///
/// This type implements [`futures::Stream`] and [`futures::TryStream`], therefore it can be used with all the adapters
/// provided by [`futures::StreamExt`] and [`futures::TryStreamExt`]:
///
/// ## Iterate over the pending messages
/// ```
/// use iota_streams_app_channels::{
///     api::tangle::futures::TryStreamExt,
///     Address,
///     Bytes,
///     GenericUserBuilder,
///     GenericUserIdentity,
///     MessageContent,
///     Tangle,
///     UnwrappedMessage,
/// };
///
/// #
/// # use std::cell::RefCell;
/// # use std::rc::Rc;
/// # use iota_streams_app_channels::api::BucketTransport;
/// # use iota_streams_core::Result;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let test_transport = Rc::new(RefCell::new(BucketTransport::new()));
/// #
/// let author_seed = "cryptographically-secure-random-author-seed";
/// let author_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let author_transport = test_transport.clone();
/// #
/// let mut author = GenericUserBuilder::new()
///     .with_identity(GenericUserIdentity::new(author_seed))
///     .with_transport(author_transport)
///     .build()?;
///
/// let subscriber_seed = "cryptographically-secure-random-subscriber-seed";
/// let subscriber_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let subscriber_transport = test_transport.clone();
/// #
/// let mut subscriber = GenericUserBuilder::new()
///     .with_identity(GenericUserIdentity::new(subscriber_seed))
///     .with_transport(subscriber_transport)
///     .build()?;
///
/// let announcement_link = author.send_announce().await?;
/// subscriber.receive_announcement(&announcement_link).await?;
/// let subscription_link = subscriber.send_subscribe(&announcement_link).await?;
/// author.receive_subscribe(&subscription_link).await?;
/// let (keyload_link, sequence_link) = author.send_keyload_for_everyone(&announcement_link).await?;
/// let (first_packet_link, sequence_link) = author
///     .send_signed_packet(&keyload_link, &b"public payload".into(), &b"masked payload".into())
///     .await?;
/// let (second_packet_link, sequence_link) = author
///     .send_signed_packet(
///         &first_packet_link,
///         &b"another public payload".into(),
///         &b"another masked payload".into(),
///     )
///     .await?;
///
/// #
/// # let mut n = 0;
/// #
/// let mut messages = subscriber.messages();
/// while let Some(msg) = messages.try_next().await? {
///     println!(
///         "New message!\n\tPublic: {}\n\tMasked: {}\n",
///         msg.body.public_payload().and_then(Bytes::as_str).unwrap_or("None"),
///         msg.body.masked_payload().and_then(Bytes::as_str).unwrap_or("None")
///     );
/// #
/// #   n += 1;
/// #
/// }
/// #
/// # assert_eq!(n, 3);
/// # Ok(())
/// # }
/// ```
///
/// ## Collect all the pending messages into a Vector (or any other collection type)
/// ```
/// use iota_streams_app_channels::{
///     api::tangle::futures::TryStreamExt,
///     Address,
///     GenericUserBuilder,
///     GenericUserIdentity,
///     MessageContent,
///     Tangle,
///     UnwrappedMessage,
/// };
///
/// #
/// # use std::cell::RefCell;
/// # use std::rc::Rc;
/// # use iota_streams_app_channels::api::BucketTransport;
/// # use iota_streams_core::Result;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let test_transport = Rc::new(RefCell::new(BucketTransport::new()));
/// #
/// let author_seed = "cryptographically-secure-random-author-seed";
/// let author_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let author_transport = test_transport.clone();
/// #
/// let author_transport = test_transport.clone();
/// let mut author = GenericUserBuilder::new()
///     .with_identity(GenericUserIdentity::new(author_seed))
///     .with_transport(author_transport)
///     .build()?;
///
/// let subscriber_seed = "cryptographically-secure-random-subscriber-seed";
/// let subscriber_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let subscriber_transport = test_transport.clone();
/// #
/// # let mut subscriber = GenericUserBuilder::new()
/// #    .with_identity(GenericUserIdentity::new(subscriber_seed))
/// #    .with_transport(subscriber_transport)
/// #    .build()?;
///
/// let announcement_link = author.send_announce().await?;
/// subscriber.receive_announcement(&announcement_link).await?;
/// let subscription_link = subscriber.send_subscribe(&announcement_link).await?;
/// author.receive_subscribe(&subscription_link).await?;
/// let (keyload_link, sequence_link) = author.send_keyload_for_everyone(&announcement_link).await?;
/// let (first_packet_link, sequence_link) = author
///     .send_signed_packet(&keyload_link, &b"public payload".into(), &b"masked payload".into())
///     .await?;
/// let (second_packet_link, sequence_link) = author
///     .send_signed_packet(
///         &first_packet_link,
///         &b"another public payload".into(),
///         &b"another masked payload".into(),
///     )
///     .await?;
///
/// let messages: Vec<UnwrappedMessage> = subscriber.messages().try_collect().await?;
/// assert_eq!(
///     messages,
///     vec![
///         UnwrappedMessage::new(keyload_link, announcement_link, MessageContent::new_keyload()),
///         UnwrappedMessage::new(
///             first_packet_link,
///             keyload_link,
///             MessageContent::new_signed_packet(author.id().clone(), b"public payload", b"masked payload")
///         ),
///         UnwrappedMessage::new(
///             second_packet_link,
///             first_packet_link,
///             MessageContent::new_signed_packet(
///                 author.id().clone(),
///                 b"another public payload",
///                 b"another masked payload"
///             )
///         ),
///     ]
/// );
/// #
/// # Ok(())
/// # }
/// ```
///
/// ## Iterate over the channel messages indefinitely
///
/// ### Author
/// ```
/// # use core::cell::RefCell;
/// use iota_streams_app_channels::{
///     api::tangle::futures::TryStreamExt,
///     Bytes,
///     GenericUserBuilder,
///     GenericUserIdentity,
///     Tangle,
/// };
/// # use iota_streams_app_channels::api::BucketTransport;
/// # use iota_streams_core::{prelude::Rc, Result};
///
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let test_transport = Rc::new(RefCell::new(BucketTransport::new()));
/// #
/// let author_seed = "cryptographically-secure-random-author-seed";
/// let author_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let author_transport = test_transport.clone();
/// let mut author = GenericUserBuilder::new()
///     .with_identity(GenericUserIdentity::new(author_seed))
///     .with_transport(author_transport)
///     .build()?;
///
/// let announcement_link = author.send_announce().await?;
/// let shareable_announcement_link = announcement_link.to_string();
///
/// # let subscriber_seed = "cryptographically-secure-random-subscriber-seed";
/// # let subscriber_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let subscriber_transport = test_transport.clone();
/// # let mut subscriber = GenericUserBuilder::new()
/// #    .with_identity(GenericUserIdentity::new(subscriber_seed))
/// #    .with_transport(subscriber_transport)
/// #    .build()?;
/// #
/// # let announcement_link = shareable_announcement_link.parse().expect("parsing announcement link");
/// # subscriber.receive_announcement(&announcement_link).await?;
/// # let subscription_link = subscriber.send_subscribe(&announcement_link).await?;
/// #
/// # let subscriber_process = async move {
/// #
/// # let mut n = 0;
/// # let mut messages = subscriber.messages();
/// # loop {
/// #   if n >= 6 {
/// #       break;
/// #   }
/// #   if let Some(msg) = messages.try_next().await? {
/// #       println!(
/// #           "New message!\n\tPublic: {}\n\tMasked: {}\n",
/// #           msg.body.public_payload().and_then(Bytes::as_str).unwrap_or("None"),
/// #           msg.body.masked_payload().and_then(Bytes::as_str).unwrap_or("None")
/// #       );
/// #     n += 1;
/// #   }
/// # }
/// #
/// # let r: Result<()> = Ok(());
/// # r
/// # };
/// #
/// // The subscription link is provided by the subscriber once she sends the subscription message
/// let shareable_susbcription_link = "<subscription-link>";
/// # let shareable_subscription_link = subscription_link.to_string();
/// let subscription_link = shareable_subscription_link.parse().expect("parsing subscription link");
/// author.receive_subscribe(&subscription_link).await?;
/// let (keyload_link, sequence_link) = author.send_keyload_for_everyone(&announcement_link).await?;
/// let mut last_link = keyload_link;
/// #
/// # let author_process = async move {
/// #
/// loop {
/// #
/// # break;
/// # }
/// # for _ in 0..5 {
/// #
///     let (packet_link, sequence_link) = author
///         .send_signed_packet(&last_link, &b"public payload".into(), &b"masked payload".into())
///         .await?;
///     last_link = packet_link;
/// }
/// #
/// # let r: Result<()> = Ok(());
/// # r
/// # };
/// #
/// # author_process.await?;
/// # subscriber_process.await?;
/// # Ok(())
/// # }
/// #
/// ```
///
/// # Subscriber
/// ```
/// # use core::cell::RefCell;
/// use iota_streams_app_channels::{
///     api::tangle::futures::TryStreamExt,
///     Bytes,
///     GenericUserBuilder,
///     GenericUserIdentity,
///     Tangle,
/// };
/// # use iota_streams_app_channels::api::BucketTransport;
/// # use iota_streams_core::{prelude::Rc, Result};
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let test_transport = Rc::new(RefCell::new(BucketTransport::new()));
/// #
/// # let author_seed = "cryptographically-secure-random-author-seed";
/// # let author_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let author_transport = test_transport.clone();
/// #
/// # let mut author = GenericUserBuilder::new()
/// #     .with_identity(GenericUserIdentity::new(author_seed))
/// #     .with_transport(author_transport)
/// #     .build()?;
/// # let announcement_link = author.send_announce().await?;
///
/// let subscriber_seed = "cryptographically-secure-random-subscriber-seed";
/// let subscriber_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let subscriber_transport = test_transport.clone();
/// #
/// let mut subscriber = GenericUserBuilder::new()
///     .with_identity(GenericUserIdentity::new(subscriber_seed))
///     .with_transport(subscriber_transport)
///     .build()?;
///
/// // Announcement link is provided by author
/// let shareable_announcement_link = "<channel-announcement-link>";
/// # let shareable_announcement_link = announcement_link.to_string();
/// let announcement_link = shareable_announcement_link.parse().expect("parsing announcement link");
/// subscriber.receive_announcement(&announcement_link).await?;
/// let subscription_link = subscriber.send_subscribe(&announcement_link).await?;
/// let shareable_subscription_link = subscription_link.to_string();
/// #
/// # let subscriber_process = async move {
/// #
/// # let mut n = 0;
///
/// let mut messages = subscriber.messages();
/// loop {
/// #   if n >= 6 {
/// #       break;
/// #   }
///     if let Some(msg) = messages.try_next().await? {
///         println!(
///             "New message!\n\tPublic: {}\n\tMasked: {}\n",
///             msg.body.public_payload().and_then(Bytes::as_str).unwrap_or("None"),
///             msg.body.masked_payload().and_then(Bytes::as_str).unwrap_or("None")
///         );
/// #       n += 1;
///     }
/// }
/// #
/// # let r: Result<()> = Ok(());
/// # r
/// # };
/// #
/// # // The subscription link is provided by the subscriber once she sends the subscription message
/// # let subscription_link = shareable_subscription_link.parse().expect("parsing subscription link");
/// # author.receive_subscribe(&subscription_link).await?;
/// # let (keyload_link, sequence_link) = author.send_keyload_for_everyone(&announcement_link).await?;
/// # let mut last_link = keyload_link;
/// #
/// # let author_process = async move {
/// #
/// # loop {
/// #
/// # break;
/// # }
/// # for _ in 0..5 {
/// #
/// #     let (packet_link, sequence_link) = author
/// #         .send_signed_packet(&last_link, &b"public payload".into(), &b"masked payload".into())
/// #         .await?;
/// #     last_link = packet_link;
/// # }
/// #
/// # let r: Result<()> = Ok(());
/// # r
/// # };
/// #
/// # author_process.await?;
/// # subscriber_process.await?;
/// # Ok(())
/// # }
/// #
/// ```
/// ## Filter the messages of a particular branch
/// ```
/// use iota_streams_app_channels::{
///     api::tangle::futures::{
///         future,
///         StreamExt,
///         TryStreamExt,
///     },
///     Address,
///     Bytes,
///     GenericUserBuilder,
///     GenericUserIdentity,
///     MessageContent,
///     Tangle,
///     UnwrappedMessage,
/// };
///
/// #
/// # use std::cell::RefCell;
/// # use std::rc::Rc;
/// # use iota_streams_app_channels::api::BucketTransport;
/// # use iota_streams_core::Result;
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let test_transport = Rc::new(RefCell::new(BucketTransport::new()));
/// #
/// let author_seed = "cryptographically-secure-random-author-seed";
/// let author_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let author_transport = test_transport.clone();
/// #
/// let mut author = GenericUserBuilder::new()
///     .with_identity(GenericUserIdentity::new(author_seed))
///     .with_transport(author_transport)
///     .build()?;
/// let subscriber_seed = "cryptographically-secure-random-subscriber-seed";
/// let subscriber_transport = Tangle::new_from_url("https://chrysalis-nodes.iota.org");
/// #
/// # let subscriber_transport = test_transport.clone();
/// #
/// let mut subscriber = GenericUserBuilder::new()
///     .with_identity(GenericUserIdentity::new(subscriber_seed))
///     .with_transport(subscriber_transport)
///     .build()?;
///
/// let announcement_link = author.send_announce().await?;
/// subscriber.receive_announcement(&announcement_link).await?;
/// let subscription_link = subscriber.send_subscribe(&announcement_link).await?;
/// author.receive_subscribe(&subscription_link).await?;
///
/// let (first_keyload_link, _sequence_link) = author.send_keyload_for_everyone(&announcement_link).await?;
/// let (tag_first_branch_link, _sequence_link) = author
///     .send_signed_packet(&first_keyload_link, &Default::default(), &b"branch 1".into())
///     .await?;
/// let (first_packet_first_branch_link, _sequence_link) = author
///     .send_signed_packet(
///         &tag_first_branch_link,
///         &Default::default(),
///         &b"masked payload in branch 1".into(),
///     )
///     .await?;
/// let (second_packet_first_branch_link, _sequence_link) = author
///     .send_signed_packet(
///         &first_packet_first_branch_link,
///         &Default::default(),
///         &b"another masked payload in branch 1".into(),
///     )
///     .await?;
///
/// let (second_keyload_link, sequence_link) = author.send_keyload_for_everyone(&announcement_link).await?;
/// let (tag_second_branch_link, _sequence_link) = author
///     .send_signed_packet(&second_keyload_link, &Default::default(), &b"branch 2".into())
///     .await?;
/// let (first_packet_second_branch_link, sequence_link) = author
///     .send_signed_packet(
///         &tag_second_branch_link,
///         &Default::default(),
///         &b"masked payload in branch 2".into(),
///     )
///     .await?;
/// let (second_packet_second_branch_link, sequence_link) = author
///     .send_signed_packet(
///         &first_packet_second_branch_link,
///         &Default::default(),
///         &b"another masked payload in branch 2".into(),
///     )
///     .await?;
///
/// let messages: Vec<UnwrappedMessage> = subscriber
///     .messages()
///     .filter_branch(|msg| {
///         future::ok(
///             msg.body
///                 .masked_payload()
///                 .and_then(Bytes::as_str)
///                 .map(|payload| payload != "branch 2")
///                 .unwrap_or(true),
///         )
///     })
///     .skip(1) // Skip tag message
///     .try_collect()
///     .await?;
///
/// assert_eq!(
///     messages,
///     vec![
///         UnwrappedMessage::new(
///             first_packet_second_branch_link,
///             tag_second_branch_link,
///             MessageContent::new_signed_packet(author.id().clone(), b"", b"masked payload in branch 2")
///         ),
///         UnwrappedMessage::new(
///             second_packet_second_branch_link,
///             first_packet_second_branch_link,
///             MessageContent::new_signed_packet(author.id().clone(), b"", b"another masked payload in branch 2")
///         ),
///     ]
/// );
/// # Ok(())
/// # }
/// ```
/// ## Iterate until finding a particular message
/// See [Filter the Messages of a Particular Branch example](#filter-the-messages-of-a-particular-branch)
/// ## Iterate over multiple channels at the same time
/// **TODO signal synchronization using zip**
/// ## Concatenate payloads
/// **TODO accounting using fold**
/// ## Wait over multiple channels concurrently and handle the first that has a new message ready
/// **TODO RPC server using select**
///
/// # Technical Details
/// This [`Stream`] makes sure the messages are traversed in topological order (preorder). This means any parent
/// message is yielded before its childs. As a consequence, there might be multiple transport
/// calls before a message is yielded, and several messages can be accumulated in memory until their turn.
/// Therefore, some jitter might be expected, with a worst case of fetching all the messages before any is
/// yielded.
///
/// Sequence messages and unreadable messages are not yielded, as they are not considered to add any end-user value.
/// Particularly unreadable messages are optimistically considered children waiting for their parent, thus accumulated
/// in memory and reprocessed instead of being yielded.
///
/// After the last currently available message has been returned, [`Messages::next()`] returns `None`, at which point
/// the [`StreamExt`] and [`TryStreamExt`] methods will consider the [`Stream`] finished and stop iterating.
/// It is safe to continue calling [`Messages::next()`] or any method from [`StreamExt`] and [`TryStreamExt`] polling
/// for new messages.
///
/// Being a [`futures::Stream`] that fetches data from an external source, it's naturally defined as a
/// [`futures::TryStream`], which means it returns a [`Result`] wrapping the [`UnwrappedMessage`]. In the event of a
/// network failure, [`Messages::next()`] will return `Err`. It is strongly suggested that, when suitable, use the
/// methods in [`futures::TryStreamExt`] to make the error-handling much more ergonomic (with the use of `?`) and
/// shortcircuit the [`futures::Stream`] on the first error.
pub struct Messages<'a, T, F, A, AG>(PinBoxFut<'a, (MessagesState<'a, T, F, A, AG>, Option<Result<Message<A>>>)>)
where
    A: Link,
    A::Relative: Eq + Hash;

type PinBoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

struct MessagesState<'a, T, F, A, AG>
where
    A: Link,
    A::Relative: Eq + Hash,
{
    user: &'a mut User<T, F, A, AG>,
    ids_stack: Vec<(Identifier, usize)>,
    msg_queue: HashMap<A::Relative, VecDeque<(A::Relative, TransportMessage)>>,
    stage: VecDeque<(A::Relative, TransportMessage)>,
    successful_round: bool,
}

impl<'a, T, F, A, AG> MessagesState<'a, T, F, A, AG>
where
    A: Link,
    A::Relative: Eq + Hash,
{
    fn new(user: &'a mut User<T, F, A, AG>) -> Self {
        Self {
            user,
            ids_stack: Vec::new(),
            msg_queue: HashMap::new(),
            stage: VecDeque::new(),
            successful_round: false,
        }
    }

    /// Fetch the next message of the channel
    ///
    /// See [`Messages`] documentation and examples for more details.
    #[async_recursion(?Send)]
    async fn next(&mut self) -> Option<Result<Message<A>>>
    where
        A: Link + Display + Clone,
        A::Relative: Clone + Eq + Hash + Default,
        A::Base: Clone,
        F: PRP + Default + Clone,
        AG: for<'b> LinkGenerator<'b, A::Relative, Data = (&'b A::Base, Identifier, usize)> + Default,
        for<'b, 'c> unwrap::Context<F, &'b [u8]>: Absorb<&'c mut A::Relative>,
        T: for<'b> Transport<'b, Address = &'b A, Msg = TransportMessage>,
    {
        if let Some((relative_address, binary_msg)) = self.stage.pop_front() {
            // Drain stage if not empty...
            let address = A::from_parts(
                self.user.stream_address().as_ref()?.base().clone(),
                relative_address.clone(),
            );
            match self.user.handle_message(address, binary_msg).await {
                Ok(Message {
                    header:
                        HDF {
                            linked_msg_address: Some(linked_msg_address),
                            ..
                        },
                    content:
                        MessageContent::Orphan(Orphan {
                            // Currently ignoring cursor, as `GenericUser::handle_message()` parses the whole binary
                            // message again this redundancy is acceptable in favour of
                            // avoiding carrying over the Spongos state within `Message`
                            message: orphaned_msg,
                            ..
                        }),
                    ..
                }) => {
                    // The message might be unreadable because it's predecessor might still be pending
                    // to be retrieved from the Tangle. We could defensively check if the predecessor
                    // is already present in the state, but we don't want to couple this iterator to
                    // a memory-intensive storage. Instead, we take the optimistic approach and store
                    // the msg for later if the handling has failed.
                    self.msg_queue
                        .entry(linked_msg_address)
                        .or_default()
                        .push_back((relative_address, orphaned_msg));

                    self.next().await
                }
                Ok(message) => {
                    // Check if message has descendants pending to process and stage them for processing
                    if let Some(msgs) = self.msg_queue.remove(message.address().relative()) {
                        self.stage.extend(msgs);
                    }

                    Some(Ok(message))
                }
                // message-Handling errors are a normal execution path, just skip them
                Err(_e) => self.next().await,
            }
        } else {
            // Stage is empty, populate it with some more messages
            let (publisher, cursor) = match self.ids_stack.pop() {
                Some(id_cursor) => id_cursor,
                None => {
                    // new round
                    self.successful_round = false;
                    let mut publisher_cursors = self.user.cursors();
                    let next = publisher_cursors.next()?;
                    self.ids_stack = publisher_cursors.collect();
                    next
                }
            };
            let base_address = self.user.stream_address().as_ref()?.base();
            let rel_address = AG::default().gen((base_address, publisher, cursor + 1));
            let address = A::from_parts(base_address.clone(), rel_address);
            match self.user.transport_mut().recv_message(&address).await {
                Ok(msg) => {
                    self.stage.push_back((address.into_relative(), msg));
                    self.successful_round = true;
                    self.next().await
                }
                Err(_e) => {
                    // Message not found or network error. Right now we are not distinguishing
                    // between each case, so we must assume it's message not found.
                    // When we introduce typed error handling and are able to distinguish,
                    // Return Err(e) if error is network-related or any other transient error
                    if self.ids_stack.is_empty() && !self.successful_round {
                        // After trying all ids, none has produced an existing link, end of stream (for now...)
                        None
                    } else {
                        // At least one id is producing existing links. continue...
                        self.next().await
                    }
                }
            }
        }
    }
}

impl<'a, T, F, A, AG> Messages<'a, T, F, A, AG>
where
    A: Link + Display + Clone,
    A::Relative: Clone + Eq + Hash + Default,
    A::Base: Clone,
    F: PRP + Default + Clone,
    AG: for<'b> LinkGenerator<'b, A::Relative, Data = (&'b A::Base, Identifier, usize)> + Default,
    for<'b, 'c> unwrap::Context<F, &'b [u8]>: Absorb<&'c mut A::Relative>,
    T: for<'b> Transport<'b, Address = &'b A, Msg = TransportMessage>,
{
    pub(crate) fn new(user: &'a mut User<T, F, A, AG>) -> Self {
        let mut state = MessagesState::new(user);
        Self(Box::pin(async move {
            let r = state.next().await;
            (state, r)
        }))
    }

    pub async fn next(&mut self) -> Option<Result<Message<A>>> {
        StreamExt::next(self).await
    }

    /// Start streaming from a particular message
    ///
    /// Once that message is fetched and yielded, the returned [`Stream`] will yield only
    /// descendants of that message.
    ///
    ///  See [example in `Messages` docs](struct.Messages.html#filter-the-messages-of-a-particular-branch)
    /// for more details.
    pub fn filter_branch<Fut>(
        self,
        predicate: impl FnMut(&Message<A>) -> Fut + 'a,
    ) -> impl Stream<Item = Result<Message<A>>> + 'a
    where
        Fut: Future<Output = Result<bool>> + 'a,
        Self: TryStream<Ok = Message<A>, Error = anyhow::Error>,
    {
        self.try_skip_while(predicate)
            .scan(None, |branch_last_address, msg| {
                future::ready(Some(msg.map(|msg| {
                    let msg_linked_address = msg.header().linked_msg_address().as_ref()?;
                    let branch_last_address = branch_last_address.get_or_insert_with(|| msg_linked_address.clone());
                    if msg_linked_address == branch_last_address {
                        *branch_last_address = msg.address().relative().clone();
                        Some(msg)
                    } else {
                        None
                    }
                })))
            })
            .try_filter_map(future::ok)
    }
}

impl<'a, T, F, A, AG> From<&'a mut User<T, F, A, AG>> for Messages<'a, T, F, A, AG>
where
    A: Link + Display + Clone,
    A::Relative: Clone + Eq + Hash + Default,
    A::Base: Clone,
    F: PRP + Default + Clone,
    AG: for<'b> LinkGenerator<'b, A::Relative, Data = (&'b A::Base, Identifier, usize)> + Default,
    for<'b, 'c> unwrap::Context<F, &'b [u8]>: Absorb<&'c mut A::Relative>,
    T: for<'b> Transport<'b, Address = &'b A, Msg = TransportMessage>,
{
    fn from(user: &'a mut User<T, F, A, AG>) -> Self {
        Self::new(user)
    }
}

impl<'a, T, F, A, AG> Stream for Messages<'a, T, F, A, AG>
where
    A: Link + Display + Clone,
    A::Relative: Clone + Eq + Hash + Default,
    A::Base: Clone,
    F: PRP + Default + Clone,
    AG: for<'b> LinkGenerator<'b, A::Relative, Data = (&'b A::Base, Identifier, usize)> + Default,
    for<'b, 'c> unwrap::Context<F, &'b [u8]>: Absorb<&'c mut A::Relative>,
    T: for<'b> Transport<'b, Address = &'b A, Msg = TransportMessage>,
{
    type Item = Result<Message<A>>;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.0.as_mut().poll(ctx) {
            Poll::Ready((mut state, result)) => {
                self.set(Messages(Box::pin(async move {
                    let r = state.next().await;
                    (state, r)
                })));
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::rc::Rc;
    use core::cell::RefCell;

    use anyhow::Result;

    use spongos::KeccakF1600;
    use LETS::{
        id::Ed25519,
        link::Address,
        transport::bucket,
    };

    use crate::api::{
        message::{
            Message,
            MessageContent::{
                Keyload,
                SignedPacket,
            },
        },
        user::User,
    };

    type Transport = Rc<RefCell<bucket::Client>>;

    #[tokio::test]
    async fn messages_awake_pending_messages_link_to_them_even_if_their_content_is_unreadable() -> Result<()> {
        let p = b"payload";
        let (mut author, mut subscriber1, announcement_link, transport) = author_subscriber_fixture().await?;

        let keyload_1 = author.send_keyload_for_all_rw(announcement_link.relative()).await?;
        subscriber1.sync().await?;
        let packet_1 = subscriber1
            .send_signed_packet(keyload_1.to_address().relative(), &p, &p)
            .await?;
        // This packet will never be readable by subscriber2. However, she will still be able to progress through the
        // next messages
        let packet_2 = subscriber1
            .send_signed_packet(packet_1.to_address().relative(), &p, &p)
            .await?;

        let mut subscriber2 = subscriber_fixture("subscriber2", &mut author, announcement_link, transport).await?;

        author.sync().await?;

        // This packet has to wait in the `Messages::msg_queue` until `packet` is processed
        let keyload_2 = author.send_keyload_for_all_rw(packet_2.to_address().relative()).await?;

        subscriber1.sync().await?;
        let last_signed_packet = subscriber1
            .send_signed_packet(keyload_2.to_address().relative(), &p, &p)
            .await?;

        let msgs = subscriber2.fetch_next_messages().await?;
        assert_eq!(3, msgs.len()); // keyload_1, keyload_2 and last signed packet
        assert!(matches!(
            msgs.as_slice(),
            &[
                Message {
                    address: address_1,
                    content: Keyload(..),
                    ..
                },
                Message {
                    address: address_2,
                    content: Keyload(..),
                    ..
                },
                Message {
                    address: address_3,
                    content: SignedPacket(..),
                    ..
                }
            ]
            if address_1 == keyload_1.to_address()
            && address_2 == keyload_2.to_address()
            && address_3 == last_signed_packet.to_address()
        ));
        Ok(())
    }

    /// Prepare a simple scenario with an author, a subscriber, a channel announcement and a bucket transport
    async fn author_subscriber_fixture() -> Result<(User<Transport>, User<Transport>, Address, Transport)> {
        let transport = Rc::new(RefCell::new(bucket::Client::new()));
        let mut author = User::builder()
            .with_identity(Ed25519::from_seed("author"))
            .with_transport(transport.clone())
            .build()?;
        author.create_stream(10)?;
        let announcement = author.announce().await?;
        let subscriber =
            subscriber_fixture("subscriber", &mut author, announcement.to_address(), transport.clone()).await?;
        Ok((author, subscriber, announcement.to_address(), transport))
    }

    async fn subscriber_fixture(
        seed: &str,
        author: &mut User<Transport>,
        announcement_link: Address,
        transport: Transport,
    ) -> Result<User<Transport>> {
        let mut subscriber = User::builder()
            .with_identity(Ed25519::from_seed(seed))
            .with_transport(transport)
            .build()?;
        subscriber.receive_message(announcement_link).await?;
        let subscription = subscriber.subscribe(announcement_link.relative()).await?;
        author.receive_message(subscription.to_address()).await?;
        Ok(subscriber)
    }
}
