//! Requeuest is a library for queueing the sending of HTTP requests.
#![deny(missing_docs)]

pub mod error;
pub(crate) mod job;

pub use reqwest::{header::HeaderMap, Method};
use sqlx::{Pool, Postgres};
use sqlxmq::JobRegistry;
pub use url::Url;
pub use uuid::Uuid;

pub use job::Request;

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

/// Adds the given request to the queue on the given channel. Returns the uuid of the spawned job.
pub async fn request(
	pool: &Pool<Postgres>,
	channel: &'static str,
	request: Request,
) -> Result<Uuid, sqlx::Error> {
	job::http
		.builder()
		.set_json(&request)
		.unwrap()
		.set_channel_name(channel)
		.spawn(pool)
		.await
}

/// Adds a `GET` request for the given url to the queue on the specified channel. This is a
/// convenience method for invoking [`request`](crate::request).
pub async fn get<'a>(
	pool: &'a Pool<Postgres>,
	channel: &'static str,
	url: Url,
	headers: HeaderMap,
) -> Result<Uuid, sqlx::Error> {
	let req = Request {
		url,
		body: None,
		method: Method::GET,
		headers,
	};
	request(pool, channel, req).await
}

/// Adds a `POST` request for the given url with the given body to the queue on the given channel.
/// This is a convenience method for invoking [`request`](crate::request).
pub async fn post<'a>(
	pool: &'a Pool<Postgres>,
	channel: &'static str,
	url: Url,
	headers: HeaderMap,
	body: Vec<u8>,
) -> Result<Uuid, sqlx::Error> {
	let req = Request {
		url,
		body: Some(body),
		method: Method::POST,
		headers,
	};
	request(pool, channel, req).await
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
	request(pool, channel, req).await
}

/// Adds the given request to the queue, and awaits until the request has been successfully
/// completed.
pub async fn response(
	pool: &Pool<Postgres>,
	channel: &'static str,
	request: Request,
) -> Result<reqwest::Response, Box<dyn std::error::Error>> {
	// Put a sender in the sender map so the job can use it
	let uuid = Uuid::new_v4();
	let (sender, receiver) = tokio::sync::oneshot::channel();
	let sender_map = job::response_senders().await;
	let mut lock = match sender_map.lock() {
		Ok(lock) => lock,
		// TODO: log/recover from poisoning better
		Err(poisoned) => poisoned.into_inner(),
	};
	lock.insert(uuid, sender);
	drop(lock);

	// Spawn the job
	job::http_response
		.builder_with_id(uuid)
		.set_json(&request)
		.unwrap()
		.set_channel_name(channel)
		.spawn(pool)
		.await?;
	Ok(receiver.await?)
}
