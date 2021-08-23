//! Contains the definition of the request which gets (de)serialized and sent to
//! the database

use std::collections::HashSet;

use reqwest::{header::HeaderMap, Method, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;

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
	/// A set of HTTP response codes which won't cause a retry.
	#[serde(default = "default_accepted_responses")]
	pub accept_responses: HashSet<AcceptedResponse>,
}

/// The kinds of categories of response codes which a response can accept
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcceptedResponse {
	/// Accept success responses, i.e. code range 200-299.
	Success,
	/// Accept redirection responses, i.e. code range 300-399.
	Redirection,
	/// Accept client-side error responses, i.e. code range 400-499.
	ClientError,
	/// Accept server-side error responses, i.e. code range 500-599.
	ServerError,
	/// Accept a single specific response code.
	Single(u16),
	/// Accept an inclusive range of responses.
	Range(u16, u16),
}

impl AcceptedResponse {
	/// Checked whether this acceptance filter accepts the given status code.
	pub fn accepts(self, status: StatusCode) -> bool {
		match self {
			AcceptedResponse::Success => status.is_success(),
			AcceptedResponse::Redirection => status.is_redirection(),
			AcceptedResponse::ClientError => status.is_client_error(),
			AcceptedResponse::ServerError => status.is_server_error(),
			AcceptedResponse::Single(code) => status.as_u16() == code,
			AcceptedResponse::Range(min, max) => status.as_u16() >= min && status.as_u16() <= max,
		}
	}
}

fn default_accepted_responses() -> HashSet<AcceptedResponse> {
	std::array::IntoIter::new([AcceptedResponse::Success]).collect()
}

impl Request {
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
			accept_responses: default_accepted_responses(),
		}
	}

	/// Contructs a `POST` request to be sent to the given url with the given
	/// body and headers.
	pub fn post(url: Url, body: Vec<u8>, headers: HeaderMap) -> Self {
		Self {
			url,
			body: Some(body),
			method: Method::POST,
			headers,
			accept_responses: default_accepted_responses(),
		}
	}

	/// Constructs a `HEAD` request to be sent to the given url.
	pub fn head(url: Url, headers: HeaderMap) -> Self {
		Self {
			url,
			body: None,
			method: Method::HEAD,
			headers,
			accept_responses: default_accepted_responses(),
		}
	}

	/// Constructs a `DELETE` request to be sent to the given url.
	pub fn delete(url: Url, body: Option<Vec<u8>>, headers: HeaderMap) -> Self {
		Self {
			url,
			body,
			method: Method::DELETE,
			headers,
			accept_responses: default_accepted_responses(),
		}
	}

	/// Constructs a `PUT` request to be sent to the given url.
	pub fn put(url: Url, body: Vec<u8>, headers: HeaderMap) -> Self {
		Self {
			url,
			body: Some(body),
			method: Method::PUT,
			headers,
			accept_responses: default_accepted_responses(),
		}
	}

	/// Convert a reqwest request into a requeuest request.
	pub fn from_reqwest(foreign: reqwest::Request) -> Self {
		Self {
			url: foreign.url().to_owned(),
			body: foreign.body().and_then(|b| b.as_bytes()).map(|b| b.to_vec()),
			method: foreign.method().to_owned(),
			headers: foreign.headers().to_owned(),
			accept_responses: default_accepted_responses(),
		}
	}
}

#[cfg(test)]
mod tests {
	use url::Url;

	use super::Request;

	#[test]
	fn serialization() {
		let url = Url::parse("https://example.com/").unwrap();
		let body = b"Some cool data".to_vec();
		let request = Request::post(url, body, Default::default());
		let serialized = bincode::serialize(&request).unwrap();
		let deserialized: Request = bincode::deserialize(&serialized).unwrap();

		assert_eq!(request.url, deserialized.url);
		assert_eq!(request.method, deserialized.method);
		assert_eq!(request.body, deserialized.body);
		assert_eq!(request.headers, deserialized.headers);
	}
}
