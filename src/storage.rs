use async_trait::async_trait;

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
