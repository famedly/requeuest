//! Requeuest is a library for queueing the sending of HTTP requests.
#![deny(missing_docs)]

pub mod error;
pub(crate) mod job;
pub mod request;

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
) -> Result<Uuid, sqlx::Error> {
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
) -> Result<Uuid, sqlx::Error> {
	let req = Request::post(url, body, headers);
	req.spawn(pool, channel).await
}

/// Converts a request from the reqwest crate into the internal request format and adds it to the
/// request queue on the given channel. The request body will be ignored if its a stream.
pub async fn from_reqwest<'a>(
	pool: &'a Pool<Postgres>,
	channel: &'static str,
	http: reqwest::Request,
) -> Result<Uuid, sqlx::Error> {
	let req = Request {
		url: http.url().to_owned(),
		// TODO: don't ignore the stream case
		body: http.body().and_then(|b| b.as_bytes()).map(|b| b.to_vec()),
		method: http.method().to_owned(),
		headers: http.headers().to_owned(),
	};
	req.spawn(pool, channel).await
}
