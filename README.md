# Requeuest
Requeuest (pronounced "recused") is a message queue which acts as an intermediary for HTTP requests, making sure that the sent request gets successfully delivered eventually, meaning that you do not have to implement retry logic for HTTP API requests. The queue uses postgres as its store for messages, to avoid the reliability risk of a dedicated message queue service potentially being down. This comes with the trade-off that job runners become part of the library consumer's process, and that a handle to the runner has to be kept alive so jobs can run in the background, since jobs cannot be delegated to a separate runner service. The queue also cannot return data from successful requests, though this is a deliberate design decision intended to limit complexity, and can be changed if necessary.

## Getting started
Assuming you already have an sqlx connection to a database, start by getting a handle to a listener for a set of channels. This is what will execute jobs in the background. It will keep doing so until it is dropped. The handle contains a tokio `JoinHandle` you can interface with directly if needed.
```rust
let listener = requeuest::listener(&connection, "my_service").await?;
```
Once the listener has been started, you can begin spawning jobs.
```rust
requeuest::get(&connection, "my_service", Url::parse("https://example.com/_api/foo/bar"), HeaderMap::new()).await?;
```
