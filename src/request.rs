//! Contains the definition of the request which gets (de)serialized and sent to the database

use crate::{error::SpawnError, job};

use reqwest::{header::HeaderMap, Method};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use url::Url;
use uuid::Uuid;

/// An HTTP request to be sent through the job queue.
#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
	/// The url to send the request to.
	pub url: Url,
	/// The body of the request.
	#[serde(with = "base64_encode")]
	pub body: Option<Vec<u8>>,
	/// The HTTP method to connect with
	#[serde(with = "http_serde::method")]
	pub method: Method,
	/// The HTTP headers to set for the request.
	#[serde(with = "http_serde::header_map")]
	pub headers: HeaderMap,
}

impl Request {
	/// Adds the given request to the queue on the given channel. Returns the uuid of the spawned job.
	pub async fn spawn(
		self,
		pool: &Pool<Postgres>,
		channel: &'static str,
	) -> Result<Uuid, sqlx::Error> {
		job::http
			.builder()
			.set_json(&self)
			.unwrap()
			.set_channel_name(channel)
			.spawn(pool)
			.await
	}

	/// Adds the request to the queue, and awaits until the request has been successfully
	/// completed, returning the received response.
	pub async fn spawn_returning(
		self,
		pool: &Pool<Postgres>,
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
			.set_json(&self)?
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
}

mod base64_encode {
	use base64::DecodeError;
	use serde::{
		self,
		de::{Error, Unexpected},
		Deserialize, Deserializer, Serializer,
	};

	pub fn serialize<S>(body: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let body = match *body {
			Some(ref body) => body,
			None => return serializer.serialize_none(),
		};
		let mut buffer = String::with_capacity((body.len() * 4) / 3);
		base64::encode_config_buf(body, base64::STANDARD, &mut buffer);
		serializer.serialize_some(&buffer)
	}

	pub fn deserialize<'d, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
	where
		D: Deserializer<'d>,
	{
		let encoded = Option::<String>::deserialize(deserializer)?;
		match encoded {
			Some(ref encoded) => {
				let mut buffer: Vec<u8> = Vec::with_capacity((encoded.len() * 3) / 4);
				if let Err(e) = base64::decode_config_buf(encoded, base64::STANDARD, &mut buffer) {
					let string = match e {
						DecodeError::InvalidByte(idx, val) => {
							format!("illegal byte {:#X} at {}", val, idx)
						}
						DecodeError::InvalidLength => String::from("invalid trailing data"),
						DecodeError::InvalidLastSymbol(val, idx) => {
							format!("ill-formed final octet with byte {:#X} at {}", val, idx)
						}
					};
					return Err(D::Error::invalid_value(
						Unexpected::Other(&string),
						&"valid base64 string",
					));
				}

				Ok(Some(buffer))
			}
			None => Ok(None),
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
