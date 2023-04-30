use crate::Storage;
use std::{fmt::Formatter, future::Future};
use tokio::{
    sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, oneshot},
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
    sender: UnboundedSender<Command<T>>,
}

impl<T> Recorder<T>
where
    T: Storage + 'static + Send,
    T::Record: Send,
    T::Query: Send,
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

    pub fn with_lazy_storage(storage: impl Future<Output=T> + Send + 'static) -> Self {
        let (sender, receiver) = unbounded_channel();
        let join_handle = tokio::spawn(async {
            let actor = Actor::new(storage.await, receiver);
            actor.run().await
        });
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
            .send(Command::Save(record))
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

    /// All the records stored in the internal storage.
    pub async fn records(&self, query: T::Query) -> Vec<T::Record> {
        let (sender, receiver) = oneshot::channel();
        self.sender.send(Command::Load(sender, query)).expect("Receiver must not be closed");
        receiver.await.expect("The sender must not be dropped")
    }
}

/// Asynchronously spawned by [`Recorder`] in order to persist records
struct Actor<T: Storage> {
    storage: T,
    receiver: UnboundedReceiver<Command<T>>,
}

impl<T> Actor<T>
where
    T: Storage,
{
    pub fn new(storage: T, receiver: UnboundedReceiver<Command<T>>) -> Self {
        Self { storage, receiver }
    }

    pub async fn run(mut self) -> T {
        // If messages come in fast, we do not send them one by one, but rather collect all since
        // the last call to save in one bulk;
        let mut bulk = Vec::new();
        let mut current = self.receiver.recv().await;
        while let Some(command) = current.take() {
            let next = match command {
                Command::Save(record) => {
                    bulk.push(record);
                    // Push all immediatly available records into the next bulk, until it would
                    // block again, or we would have to serve a load command.
                    let next = loop {
                        match self.receiver.try_recv() {
                            Ok(Command::Save(record)) => bulk.push(record),
                            Ok(other) => break Some(other),
                            Err(_) => break None,
                        }
                    };
                    self.storage.save(&mut bulk).await;
                    bulk.clear();
                    next
                },
                Command::Load(sender, query) => {
                    // Fetch records ...
                    let records = self.storage.load(query).await;
                    // ... and answer sender. This might fail, but if the sender is dropped and
                    // stopped, caring, so do we. Let's drop the result.
                    let _ = sender.send(records);
                    // We did not peek ahead, so we do not know the next command.
                    None
                },
            };
            // Use next or wait for next event
            current = if next.is_none() {
                // Wait for the next event, can block. If none this means recorder has been dropped
                // and we terminate this loop.
                self.receiver.recv().await
            } else {
                // We already know the next event to process, since we had to peek ahead.
                next
            };
        }
        self.storage
    }
}

/// Message send from recorder to actor. Allowes for custom debug implementation lifting the
/// limitation that `T` has to be `Debug`.
enum Command<T: Storage> {
    /// Save record T to the storage backend
    Save(T::Record),
    /// Load all records from the storage. Use the sender to return them back to the caller.
    Load(oneshot::Sender<Vec<T::Record>>, T::Query),
}

/// Custom implementation of debug for Message, which does not rely on the record type `T` to be
/// debug itstelf.
impl<T> std::fmt::Debug for Command<T> where T: Storage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Save(_) => f.debug_tuple("Save").finish(),
            Command::Load(..) => f.debug_tuple("Load").finish(),
        }
    }
}
