//! Requeuest is a library for queueing the sending of HTTP requests.
//!
//! ## Getting started
//! Assuming you already have an `sqlx` connection to a postgres database, you will first need to
//! run migrations so the needed tables and SQL functions can get set up on your postgres database.
//! ```no_run
//! # async fn test(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
//! requeuest::migrate(&pool).await?;
//! # Ok(())
//! # }
//! ```
//! Once that's taken care of, start by getting a handle to a listener for a set of channels. This
//! is what will execute jobs in the background. It will keep doing so until it is dropped.
//! The handle contains a tokio `JoinHandle` you can interface with directly if needed.
//! ```no_run
//! # async fn test(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
//! let listener = requeuest::listener(&pool, &["my_service"]).await?;
//! # Ok(())
//! # }
//! ```
//! After the listener has been started, you can begin spawning jobs. Here we send a get request to
//! an example address:
//! ```no_run
//! use requeuest::{HeaderMap, Request, Url};
//!
//! # async fn test(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), Box<dyn std::error::Error>> {
//! Request::get(Url::parse("https://example.com/_api/foo/bar")?, HeaderMap::new())
//!     .spawn(&pool, "my_service")
//!     .await?;
//! # Ok(())
//! # }
//! ```
//! You can also also get the response back from a successfully delivered request.
//! ```no_run
//! use requeuest::{HeaderMap, Request, Url};
//!
//! # async fn test(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), Box<dyn std::error::Error>> {
//! let response = Request::post(Url::parse("https://example.com/_api/bar/foo")?, Vec::from("some data"), HeaderMap::new())
//!     .spawn_returning(&pool, "my_service")
//!     .await?;
//! # Ok(())
//! # }
//! ```
//! Note that the `spawn_returning` method *will* wait indefinitely (or to be precise, roughly
//! 10^293 years) until a successful response is received, so this will wait forever if a request
//! is sent to e.g. an unregistered domain, or sends data to an API which will always result in a
//! non-200 response code.

#![deny(missing_docs)]

pub mod error;
pub(crate) mod job;
pub mod request;

use error::SpawnError;
pub use request::Request;

pub use reqwest::{header::HeaderMap, Method};
use sqlx::{Pool, Postgres};
use sqlxmq::JobRegistry;
pub use url::Url;
pub use uuid::Uuid;

/// Runs the SQL migrations this library needs.
pub async fn migrate(pool: &Pool<Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
	sqlx::migrate!().run(pool).await
}

/// Spawns a listener which runs jobs on the given channels until the returned handle is dropped.
/// Alternatively, the `JoinHandle` contained in the returned handle can be explicitly joined.
pub async fn listener(
	pool: &Pool<Postgres>,
	channels: &[&str],
) -> Result<sqlxmq::OwnedHandle, sqlx::Error> {
	let registry = JobRegistry::new(&[job::http, job::http_response]);
	registry
		.runner(pool)
		.set_channel_names(channels)
		.run()
		.await
}

/// Adds a `GET` request for the given url to the queue on the specified channel.
pub async fn get<'a>(
	pool: &'a Pool<Postgres>,
	channel: &'static str,
	url: Url,
	headers: HeaderMap,
) -> Result<Uuid, SpawnError> {
	let req = Request::get(url, headers);
	req.spawn(pool, channel).await
}

/// Adds a `POST` request for the given url with the given body to the queue on the specified
/// channel.
pub async fn post<'a>(
	pool: &'a Pool<Postgres>,
	channel: &'static str,
	url: Url,
	headers: HeaderMap,
	body: Vec<u8>,
) -> Result<Uuid, SpawnError> {
	let req = Request::post(url, body, headers);
	req.spawn(pool, channel).await
}

/// Converts a request from the reqwest crate into the internal request format and adds it to the
/// request queue on the given channel. The request body will be ignored if its a stream.
pub async fn from_reqwest<'a>(
	pool: &'a Pool<Postgres>,
	channel: &'static str,
	http: reqwest::Request,
) -> Result<Uuid, SpawnError> {
	let req = Request {
		url: http.url().to_owned(),
		// TODO: don't ignore the stream case
		body: http.body().and_then(|b| b.as_bytes()).map(|b| b.to_vec()),
		method: http.method().to_owned(),
		headers: http.headers().to_owned(),
	};
	req.spawn(pool, channel).await
}
