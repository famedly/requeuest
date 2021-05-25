# Requeuest
Requeuest (pronounced "recused") is a message queue which acts as an intermediary for HTTP requests, making sure that the sent request gets successfully delivered eventually, meaning that you do not have to implement retry logic for HTTP API requests. The queue uses the [`sqlxmq`] crate to make postgres as its store for messages, which avoids the reliability risk of a dedicated message queue service potentially being down. This comes with the trade-off that job runners become part of the library consumer's process, and that a handle to the runner has to be kept alive so jobs can run in the background, since jobs cannot be delegated to a separate runner service.

## Getting started
This brief guide assumes you already have a `sqlx` connection to your postgres database. First you will need to run migrations so the needed tables and SQL functions can get set up on your postgres database.
```rust
requeuest::migrate(&pool).await?
```
Once that's taken care of, start by getting a handle to a listener for a set of channels. This is what will execute jobs in the background. It will keep doing so until it is dropped. The handle contains a tokio `JoinHandle` you can interface with directly if needed.
```rust
let listener = requeuest::listener(&pool, &["my_service"]).await?;
```
After the listener has been started, you can begin spawning jobs. Here we send a get request to an example address:
```rust
use requeuest::{HeaderMap, Request, Url};

Request::get(Url::parse("https://example.com/_api/foo/bar")?, HeaderMap::new())
	.spawn(&pool, "my_service")
	.await?;
```
You can also also get the response back from a successfully delivered request.
```rust
use requeuest::{HeaderMap, Request, Url};

let response = Request::post(Url::parse("https://example.com/_api/bar/foo")?, Vec::from(b"some data"), HeaderMap::new())
	.spawn_returning(&pool, "my_service")
	.await?;
```
Note that the `spawn_returning` method *will* wait indefinitely until a successful response is received, so this will wait forever if a request is sent to e.g. an unregistered domain, or sends data to an API which will always result in a non-200 response code.

[`sqlxmq`]: https://docs.rs/sqlxmq
