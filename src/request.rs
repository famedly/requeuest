//! Contains the definition of the request which gets (de)serialized and sent to the database

use crate::{error::SpawnError, job};

use reqwest::{header::HeaderMap, Method};
use serde::{Deserialize, Serialize};
use sqlx::Postgres;
use url::Url;
use uuid::Uuid;

/// An HTTP request to be sent through the job queue.
#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
	/// The url to send the request to.
	pub url: Url,
	/// The body of the request.
	pub body: Option<Vec<u8>>,
	/// The HTTP method to connect with
	#[serde(with = "http_serde::method")]
	pub method: Method,
	/// The HTTP headers to set for the request.
	#[serde(with = "http_serde::header_map")]
	pub headers: HeaderMap,
}

impl Request {
	/// Adds the given request to the queue on the specified channel using the given executor.
	/// Returns the uuid of the spawned job. In most cases you probably want to use
	/// [`Client::spawn`](crate::Client::spawn) instead.
	pub async fn spawn_with<'a, E: sqlx::Executor<'a, Database = Postgres>>(
		&'a self,
		pool: E,
		channel: &'static str,
	) -> Result<Uuid, SpawnError> {
		let uuid = job::http
			.builder()
			.set_raw_bytes(&bincode::serialize(self)?)
			.set_channel_name(channel)
			.set_retries(100_000)
			.spawn(pool)
			.await?;
		Ok(uuid)
	}

	/// Adds the request to the queue using the given executor, and awaits until the request has
	/// been successfully completed, returning the received response. In most cases you probably
	/// want to use [`Client::spawn`](crate::Client::spawn) instead.
	pub async fn spawn_returning_with<'a, E: sqlx::Executor<'a, Database = Postgres>>(
		&'a self,
		pool: E,
		channel: &'static str,
	) -> Result<reqwest::Response, SpawnError> {
		// Put a sender in the sender map so the job can use it
		let uuid = Uuid::new_v4();
		let (sender, receiver) = tokio::sync::oneshot::channel();
		job::response_senders()
			.await
			.lock()
			.unwrap()
			.insert(uuid, sender);

		// Spawn the job
		job::http_response
			.builder_with_id(uuid)
			.set_raw_bytes(&bincode::serialize(self)?)
			.set_channel_name(channel)
			.spawn(pool)
			.await?;
		Ok(receiver.await?)
	}

	/// Constructs a `GET` request to the given address.
	///
	/// # Example
	/// ```
	/// # use url::Url;
	/// # use requeuest::Request;
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// Request::get(Url::parse("https://example.com")?, Default::default());
	/// # Ok(())
	/// # }
	/// ```
	pub fn get(url: Url, headers: HeaderMap) -> Self {
		Self {
			url,
			body: None,
			method: Method::GET,
			headers,
		}
	}

	/// Contructs a `POST` request to be sent to the given url with the given body and headers.
	pub fn post(url: Url, body: Vec<u8>, headers: HeaderMap) -> Self {
		Self {
			url,
			body: Some(body),
			method: Method::POST,
			headers,
		}
	}

	/// Constructs a `HEAD` request to be sent to the given url.
	pub fn head(url: Url, headers: HeaderMap) -> Self {
		Self {
			url,
			body: None,
			method: Method::HEAD,
			headers,
		}
	}

	/// Constructs a `DELETE` request to be sent to the given url.
	pub fn delete(url: Url, body: Option<Vec<u8>>, headers: HeaderMap) -> Self {
		Self {
			url,
			body,
			method: Method::DELETE,
			headers,
		}
	}

	/// Constructs a `PUT` request to be sent to the given url.
	pub fn put(url: Url, body: Vec<u8>, headers: HeaderMap) -> Self {
		Self {
			url,
			body: Some(body),
			method: Method::PUT,
			headers,
		}
	}

	/// Convert a reqwest request into a requeuest request.
	pub fn from_reqwest(foreign: reqwest::Request) -> Self {
		Self {
			url: foreign.url().to_owned(),
			body: foreign
				.body()
				.and_then(|b| b.as_bytes())
				.map(|b| b.to_vec()),
			method: foreign.method().to_owned(),
			headers: foreign.headers().to_owned(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::Request;

	use url::Url;

	#[test]
	fn serialization() {
		let url = Url::parse("https://example.com/").unwrap();
		let body = b"Some cool data".to_vec();
		let request = Request::post(url, body, Default::default());
		let serialized = serde_json::to_vec(&request).unwrap();
		let deserialized: Request = serde_json::from_slice(&serialized).unwrap();

		assert_eq!(request.url, deserialized.url);
		assert_eq!(request.method, deserialized.method);
		assert_eq!(request.body, deserialized.body);
		assert_eq!(request.headers, deserialized.headers);
	}
}
