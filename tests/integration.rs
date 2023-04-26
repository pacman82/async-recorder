use async_recorder::Recorder;

#[tokio::test]
async fn record_event_to_persistence_backend() {
    let record = "Hello, World!".to_owned();
    let storage = Vec::new();

    let recorder = Recorder::new(storage).await;
    recorder.save(record);
    let storage = recorder.into_storage().await;

    assert_eq!(["Hello, World!"].as_slice(), storage);
}
