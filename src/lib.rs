use std::fmt::Formatter;

use async_trait::async_trait;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

/// Persists records asynchronously
///
/// This interface is fire and forget, which makes it synchronous. It will not wait for the record
/// to be persisted. Recorder takes ownership of an actor running asynchronously, storing anything
/// send to it.
pub struct Recorder<T: Storage> {
    /// We need the handle to make sure we join the actor before our recorder goes out of scope.
    join_handle: JoinHandle<T>,
    sender: UnboundedSender<Message<T::Record>>,
}

impl<T> Recorder<T>
where
    T: Storage + 'static + Send,
    T::Record: Send,
{
    pub async fn new(storage: T) -> Self {
        let (sender, receiver) = unbounded_channel();
        let actor = Actor::new(storage, receiver);
        let join_handle = tokio::spawn(actor.run());
        Self {
            join_handle,
            sender,
        }
    }

    /// Sends the record to the internal actor for storage. This function will return immediatly.
    /// the record might only be persisted later.
    pub fn save(&self, record: T::Record) {
        self.sender
            .send(Message(record))
            .expect("Receiver must not be closed.")
    }

    /// Stop the actor, deconstruct the recorder. Yields access to the underlying storage.
    pub async fn into_storage(self) -> T {
        // Close sender, so we stop sending messages and `Actor::run`.
        drop(self.sender);
        // Now that actor run nows it should terminate, we wait for it.
        self.join_handle
            .await
            .expect("Recorder actor thread must always be able to join")
    }
}

/// Asynchronously spawned by [`Recorder`] in order to persist records
struct Actor<T: Storage> {
    storage: T,
    receiver: UnboundedReceiver<Message<T::Record>>,
}

impl<T> Actor<T>
where
    T: Storage,
{
    pub fn new(storage: T, receiver: UnboundedReceiver<Message<T::Record>>) -> Self {
        Self { storage, receiver }
    }

    pub async fn run(mut self) -> T {
        // If messages come in fast, we do not send them one by one, but rather collect all since
        // the last call to save in one bulk;
        let mut bulk = Vec::new();
        // Insert records until channel is closed.
        while let Some(Message(record)) = self.receiver.recv().await {
            bulk.push(record);
            // Push records into bulk, until it would block again.
            while let Ok(Message(record)) = self.receiver.try_recv() {
                bulk.push(record);
            }
            self.storage.save(&mut bulk).await;
            bulk.clear();
        }
        self.storage
    }
}

/// Message send from recorder to actor. allowes for custom debug implementation.
struct Message<T>(T);

/// Custom implementation of debug for Message, which does not rely on the record type `T` to be
/// debug itstelf.
impl<T> std::fmt::Debug for Message<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Record").finish()
    }
}

/// Can save records asynchronously
#[async_trait]
pub trait Storage {
    /// Records saved in the storage
    type Record;

    /// Saves all the records to the persistence backend. Note that this method is infallible. This
    /// implies that the responsibility of handling errors lies with the implementation of this
    /// trait. So it is up to the implementation to decide how often to retry before (if ever)
    /// giving up. What to log and so on.
    ///
    /// `records` contains all the records which are to be persisted with this call to save. The
    /// records are passed in a `Vec` rather than in a single call to enable bulk insertion. They
    /// are also passed in a `Vec` rather than a slice (`&[Record]`) in order to enable taking
    /// ownership of each record and avoid cloning. Finally it is a `&mut Vec` rather than a buffer
    /// so we can reuse it, without having to reallocate it a lot during the lifetime of our
    /// application.
    async fn save(&mut self, records: &mut Vec<Self::Record>);
}

/// This implementation is usefull for using as a fake for testing. In production you are more
/// likely want to talk to a database.
#[async_trait]
impl<T> Storage for Vec<T>
where
    T: Send,
{
    type Record = T;

    async fn save(&mut self, records: &mut Vec<T>) {
        self.append(records);
    }
}
