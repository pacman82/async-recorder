use crate::Storage;
use std::fmt::Formatter;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

/// Persists records asynchronously.
///
/// You may want to use this instead of directly calling your persistence backend if you do not want
/// to wait for the record to be persisted, in the handler which created the record. To achieve this
/// Recoder spawns an actor to which all records are sent immediatly. The actor when uses the
/// [`Storage`] trait to talk to your persistence backend.
///
/// Recorder takes ownership of an actor and the green thread it is running in.
pub struct Recorder<T: Storage> {
    /// We need the handle to make sure we join the actor before our recorder goes out of scope.
    join_handle: JoinHandle<T>,
    /// We choose an unbounded sender since we want to talk from sync to async code without waiting
    /// for the persistence backend to catch up.
    sender: UnboundedSender<Command<T::Record>>,
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

    /// Sends the record to the internal actor for storage. This interface is fire and forget. It
    /// will not wait for the record to be actually persisted, just place it in the channel for the
    /// actor to pick up. This is why this method is both synchronous and non blocking.
    pub fn save(&self, record: T::Record) {
        self.sender
            .send(Command(record))
            .expect("Receiver must not be closed.")
    }

    /// Stop accepting new records to save, persist the ones send so far.
    ///
    /// Gives back ownership of the underlying storage.
    pub async fn close(self) -> T {
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
    receiver: UnboundedReceiver<Command<T::Record>>,
}

impl<T> Actor<T>
where
    T: Storage,
{
    pub fn new(storage: T, receiver: UnboundedReceiver<Command<T::Record>>) -> Self {
        Self { storage, receiver }
    }

    pub async fn run(mut self) -> T {
        // If messages come in fast, we do not send them one by one, but rather collect all since
        // the last call to save in one bulk;
        let mut bulk = Vec::new();
        // Insert records until channel is closed.
        while let Some(Command(record)) = self.receiver.recv().await {
            bulk.push(record);
            // Push records into bulk, until it would block again.
            while let Ok(Command(record)) = self.receiver.try_recv() {
                bulk.push(record);
            }
            self.storage.save(&mut bulk).await;
            bulk.clear();
        }
        self.storage
    }
}

/// Message send from recorder to actor. Allowes for custom debug implementation lifting the
/// limitation that `T` has to be `Debug`.
struct Command<T>(T);

/// Custom implementation of debug for Message, which does not rely on the record type `T` to be
/// debug itstelf.
impl<T> std::fmt::Debug for Command<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Record").finish()
    }
}
