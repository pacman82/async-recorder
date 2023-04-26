use std::{mem::swap, sync::Arc};

use async_recorder::{Recorder, Storage};
use async_trait::async_trait;
use tokio::sync::Mutex;

#[tokio::test]
async fn record_event_to_persistence_backend() {
    let record = "Hello, World!".to_owned();
    let storage = Vec::new();

    let recorder = Recorder::new(storage).await;
    recorder.save(record);
    let storage = recorder.close().await;

    assert_eq!(["Hello, World!"].as_slice(), storage);
}

#[tokio::test]
async fn persist_events_in_bulk() {
    let bulks = Arc::new(Mutex::new(Vec::new()));
    let storage = StorageSpy::new(bulks.clone());

    let recorder = Recorder::new(storage).await;
    {
        // Keep guard to bulks, so spy can not persist until it is cleared
        let _guard = bulks.lock().await;
        recorder.save("first");
        recorder.save("second");
    }
    // Now the guard has been freed. Storage can now persist immediatly. Wait for this to be
    // fininshed.
    let _ = recorder.close().await;

    // Verify that there is one bulk with two entries.
    let bulks = bulks.lock().await;
    assert_eq!(1, bulks.len());
    assert_eq!(["first", "second"].as_slice(), bulks[0])
}

/// Makes a copy of each received bulk.
struct StorageSpy<T> {
    /// Make this Arc Mutex, so we can block saving and observe bulk behaviour
    bulks: Arc<Mutex<Vec<Vec<T>>>>,   
}

impl<T> StorageSpy<T> {
    fn new(bulks: Arc<Mutex<Vec<Vec<T>>>>) -> Self {
        StorageSpy { bulks }
    }
}

#[async_trait]
impl<T> Storage for StorageSpy<T> where T: Send {
    type Record = T;

    async fn save(&mut self, records: &mut Vec<T>) {
        let mut tmp = Vec::new();
        swap(&mut tmp, records);
        self.bulks.lock().await.push(tmp);
    }
}