//! Contains the definition of the request which gets (de)serialized and sent to
//! the database

use std::{collections::HashSet, convert::TryInto};

use reqwest::{header::HeaderMap, Method, StatusCode};
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
use url::Url;

/// An HTTP request to be sent through the job queue.
#[derive(Serialize, Deserialize, Debug, TypedBuilder)]
#[must_use]
pub struct Request {
	/// The url to send the request to.
	pub url: Url,
	/// The body of the request.
	#[builder(default, setter(strip_option, into))]
	pub body: Option<Vec<u8>>,
	/// The HTTP method to connect with
	#[serde(with = "http_serde::method")]
	pub method: Method,
	/// The HTTP headers to set for the request.
	#[serde(with = "http_serde::header_map")]
	#[builder(default)]
	pub headers: HeaderMap,
	/// A set of HTTP response codes which won't cause a retry.
	#[serde(default = "default_accepted_responses")]
	#[builder(default=default_accepted_responses())]
	pub accept_responses: HashSet<AcceptedResponse>,
}

/// The kinds of categories of response codes which a response can accept
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcceptedResponse {
	/// Accept information responses, i.e. code range 100-199
	Informational,
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
	/// Check whether this acceptance filter accepts the given status code.
	#[must_use]
	pub fn accepts(self, status: StatusCode) -> bool {
		match self {
			AcceptedResponse::Informational => status.is_informational(),
			AcceptedResponse::Success => status.is_success(),
			AcceptedResponse::Redirection => status.is_redirection(),
			AcceptedResponse::ClientError => status.is_client_error(),
			AcceptedResponse::ServerError => status.is_server_error(),
			AcceptedResponse::Single(code) => status.as_u16() == code,
			AcceptedResponse::Range(min, max) => status.as_u16() >= min && status.as_u16() <= max,
		}
	}
}

/// Returns the set of responses which are considered valid by default
fn default_accepted_responses() -> HashSet<AcceptedResponse> {
	IntoIterator::into_iter([AcceptedResponse::Success]).into_iter().collect()
}

/// Return builder type for methods with predefined method
type WithUrlAndMethodBuilder = RequestBuilder<((Url,), (), (Method,), (), ())>;
/// Return builder type for methods with predefined method and body
type WithUrlAndBodyAndMethodBuilder =
	RequestBuilder<((Url,), (Option<Vec<u8>>,), (Method,), (), ())>;

impl Request {
	/// Constructs a `GET` request builder.
	///
	/// # Example
	/// ```
	/// # use requeuest::Request;
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// Request::get("https://example.com")?.build();
	/// # Ok(())
	/// # }
	/// ```
	pub fn get<T>(url: T) -> Result<WithUrlAndMethodBuilder, <T as TryInto<Url>>::Error>
	where
		T: TryInto<Url>,
	{
		Ok(Request::builder().method(Method::GET).url(url.try_into()?))
	}

	/// Constructs a `HEAD` request builder.
	pub fn head<T>(url: T) -> Result<WithUrlAndMethodBuilder, <T as TryInto<Url>>::Error>
	where
		T: TryInto<Url>,
	{
		Ok(Request::builder().method(Method::HEAD).url(url.try_into()?))
	}

	/// Constructs a `DELETE` request builder.
	pub fn delete<T>(url: T) -> Result<WithUrlAndMethodBuilder, <T as TryInto<Url>>::Error>
	where
		T: TryInto<Url>,
	{
		Ok(Request::builder().method(Method::DELETE).url(url.try_into()?))
	}

	/// Constructs a `POST` request builder with the given body
	pub fn post<T>(
		url: T,
		body: impl Into<Vec<u8>>,
	) -> Result<WithUrlAndBodyAndMethodBuilder, <T as TryInto<Url>>::Error>
	where
		T: TryInto<Url>,
	{
		Ok(Request::builder().method(Method::POST).url(url.try_into()?).body(body))
	}

	/// Constructs a `PUT` request builder with the given body
	pub fn put<T>(
		url: T,
		body: impl Into<Vec<u8>>,
	) -> Result<WithUrlAndBodyAndMethodBuilder, <T as TryInto<Url>>::Error>
	where
		T: TryInto<Url>,
	{
		Ok(Request::builder().method(Method::PUT).url(url.try_into()?).body(body))
	}

	/// Convert a reqwest request into a requeuest request.
	pub fn from_reqwest(mut foreign: reqwest::Request) -> Self {
		Self {
			url: foreign.url().clone(),
			body: foreign.body().and_then(reqwest::Body::as_bytes).map(<[_]>::to_vec),
			method: std::mem::take(foreign.method_mut()),
			headers: std::mem::take(foreign.headers_mut()),
			accept_responses: default_accepted_responses(),
		}
	}

	/// Constructs a request by converting a request builder from the `http`
	/// crate. Returns `None` if the uri or the method are missing from the
	/// builder
	#[cfg(feature = "http")]
	pub fn from_http_builder(
		foreign: http::request::Builder,
		body: Option<Vec<u8>>,
	) -> Result<Self, crate::error::ConvertError> {
		match body {
			Some(body) => Ok(Self::from_http_body(foreign.body(body)?)?),
			None => Ok(Self::from_http_empty(foreign.body(())?)?),
		}
	}

	#[cfg(feature = "http")]
	fn from_http_parts(parts: http::request::Parts) -> Result<Self, url::ParseError> {
		Ok(Self {
			url: Url::parse(&parts.uri.to_string())?,
			body: None,
			method: parts.method,
			headers: parts.headers,
			accept_responses: default_accepted_responses(),
		})
	}

	/// Convert a [`http::Request`] with a body into a requeuest request.
	#[cfg(feature = "http")]
	pub fn from_http_body<B: Into<Vec<u8>>>(
		foreign: http::Request<B>,
	) -> Result<Self, url::ParseError> {
		let (parts, body) = foreign.into_parts();
		let mut request = Self::from_http_parts(parts)?;
		request.body = Some(body.into());
		Ok(request)
	}

	/// Convert a [`http::Request`] without a body into a requeuest request
	#[cfg(feature = "http")]
	pub fn from_http_empty<B>(foreign: http::Request<B>) -> Result<Self, url::ParseError> {
		let (parts, _) = foreign.into_parts();
		Self::from_http_parts(parts)
	}
}

#[cfg(test)]
mod tests {
	#![allow(clippy::unwrap_used)]
	use reqwest::{
		header::{HeaderMap, HeaderValue, AUTHORIZATION},
		Method, StatusCode,
	};
	use url::ParseError;

	use super::Request;

	/// Convenience function to convert a u16 to status code and unwrap the
	/// result
	fn status_code(code: u16) -> StatusCode {
		StatusCode::from_u16(code).unwrap()
	}

	/// Checks that `AcceptedResponse` accepts and rejects the right status
	/// codes
	#[test]
	fn accepted_range() {
		for code in (100..1000).map(status_code) {
			use super::AcceptedResponse::*;
			let num = code.as_u16();
			assert_eq!(Informational.accepts(code), (100..200).contains(&num));
			assert_eq!(Success.accepts(code), (200..300).contains(&num));
			assert_eq!(Redirection.accepts(code), (300..400).contains(&num));
			assert_eq!(ClientError.accepts(code), (400..500).contains(&num));
			assert_eq!(ServerError.accepts(code), (500..600).contains(&num));
			assert_eq!(Range(423, 489).accepts(code), (423..=489).contains(&num));
			assert_eq!(Single(200).accepts(code), num == 200);
		}
	}

	#[test]
	fn serialization() {
		let request =
			Request::post("https://example.com/", b"Some cool data".to_vec()).unwrap().build();
		let serialized = bincode::serialize(&request).unwrap();
		let deserialized: Request = bincode::deserialize(&serialized).unwrap();

		assert_eq!(request.url, deserialized.url);
		assert_eq!(request.method, deserialized.method);
		assert_eq!(request.body, deserialized.body);
		assert_eq!(request.headers, deserialized.headers);
	}

	#[test]
	fn convert_reqwest() {
		let mut foreign = reqwest::Request::new(Method::POST, "https://foo.bar/".parse().unwrap());
		foreign.headers_mut().insert(AUTHORIZATION, HeaderValue::from_static("Secret"));
		*foreign.body_mut() = Some("body".into());
		let request = Request::from_reqwest(foreign);

		assert_eq!(request.url.to_string(), "https://foo.bar/", "URL mismatch");
		assert_eq!(request.method, Method::POST, "Method mismatch");
		assert_eq!(request.headers.get(AUTHORIZATION).unwrap(), &"Secret", "Header mismatch");
		assert_eq!(request.body.unwrap(), b"body", "Body mismatch");
	}

	#[test]
	fn test_constructors() {
		let get_request = Request::get("http://get.example").unwrap().build();

		assert_eq!(get_request.url.to_string(), "http://get.example/", "URL mismatch");
		assert_eq!(get_request.method, Method::GET, "Method mismatch");
		assert_eq!(get_request.headers, HeaderMap::default(), "Header mismatch");
		assert_eq!(get_request.body, None, "Body mismatch");

		let delete_request = Request::delete("https://delete.example").unwrap().build();

		assert_eq!(delete_request.url.to_string(), "https://delete.example/", "URL mismatch");
		assert_eq!(delete_request.method, Method::DELETE, "Method mismatch");
		assert_eq!(delete_request.headers, HeaderMap::default(), "Header mismatch");
		assert_eq!(delete_request.body, None, "Body mismatch");

		let head_request = Request::head("https://head.example").unwrap().build();

		assert_eq!(head_request.url.to_string(), "https://head.example/", "URL mismatch");
		assert_eq!(head_request.method, Method::HEAD, "Method mismatch");
		assert_eq!(head_request.headers, HeaderMap::default(), "Header mismatch");
		assert_eq!(head_request.body, None, "Body mismatch");

		let post_request =
			Request::post("https://post.example", b"example post".to_vec()).unwrap().build();

		assert_eq!(post_request.url.to_string(), "https://post.example/", "URL mismatch");
		assert_eq!(post_request.method, Method::POST, "Method mismatch");
		assert_eq!(post_request.headers, HeaderMap::default(), "Header mismatch");
		assert_eq!(post_request.body.unwrap(), b"example post", "Body mismatch");

		let put_request =
			Request::put("https://put.example", b"example put".to_vec()).unwrap().build();

		assert_eq!(put_request.url.to_string(), "https://put.example/", "URL mismatch");
		assert_eq!(put_request.method, Method::PUT, "Method mismatch");
		assert_eq!(put_request.headers, HeaderMap::default(), "Header mismatch");
		assert_eq!(put_request.body.unwrap(), b"example put", "Body mismatch");
	}

	#[test]
	fn test_builder() {
		let mut header_map = HeaderMap::new();
		header_map.insert("AUTHORIZATION", "secret".parse().unwrap());
		let request = Request::builder()
			.method(Method::GET)
			.url("https://foo.bar/".parse().unwrap())
			.headers(header_map.clone())
			.build();

		assert_eq!(request.url.to_string(), "https://foo.bar/", "URL mismatch");
		assert_eq!(request.method, Method::GET, "Method mismatch");
		assert_eq!(request.headers, header_map, "Header mismatch");
		assert_eq!(request.body, None, "Body mismatch");
		assert_eq!(request.accept_responses, crate::request::default_accepted_responses());

		let request = Request::builder()
			.url("https://foo.bar/".parse().unwrap())
			.method(Method::POST)
			.body("body")
			.build();

		assert_eq!(request.url.to_string(), "https://foo.bar/", "URL mismatch");
		assert_eq!(request.method, Method::POST, "Method mismatch");
		assert_eq!(request.headers, HeaderMap::new(), "Header mismatch");
		assert_eq!(request.body.unwrap(), b"body", "Body mismatch");
	}

	#[test]
	fn test_url_parse_error() {
		let parse_error = Request::delete("test.de").err().unwrap();

		assert_eq!(parse_error, ParseError::RelativeUrlWithoutBase, "Error missmatch");
	}

	#[cfg(feature = "http")]
	#[test]
	fn convert_http_builder() {
		let foreign = http::Request::post("https://foo.bar/").header(AUTHORIZATION, "Secret");
		let request = Request::from_http_builder(foreign, Some(b"body".to_vec())).unwrap();

		assert_eq!(request.url.to_string(), "https://foo.bar/", "URL mismatch");
		assert_eq!(request.method, Method::POST, "Method mismatch");
		assert_eq!(request.headers.get(AUTHORIZATION).unwrap(), &"Secret", "Header mismatch");
		assert_eq!(request.body.unwrap(), b"body", "Body mismatch");

		let bad = http::request::Builder::new();
		assert!(Request::from_http_builder(bad, None).is_err(), "Missing value guard failed");
	}

	#[cfg(feature = "http")]
	#[test]
	fn convert_http_empty() {
		let foreign = http::Request::put("https://bar.baz/")
			.header(AUTHORIZATION, "Credentials")
			.body(())
			.unwrap();
		let request = Request::from_http_empty(foreign).unwrap();

		assert_eq!(request.url.to_string(), "https://bar.baz/", "URL mismatch");
		assert_eq!(request.method, Method::PUT, "Method mismatch");
		assert_eq!(request.headers.get(AUTHORIZATION).unwrap(), &"Credentials", "Header mismatch");
		assert_eq!(request.body, None, "Body unexpectedly not empty");
	}

	#[cfg(feature = "http")]
	#[test]
	fn convert_http_body() {
		let foreign = http::Request::delete("http://web.site/thing")
			.header(AUTHORIZATION, "Bearer: l0tsofl3tters")
			.body("yeet the thing")
			.unwrap();
		let request = Request::from_http_body(foreign).unwrap();

		assert_eq!(request.url.to_string(), "http://web.site/thing", "URL mismatch");
		assert_eq!(request.method, Method::DELETE, "Method mismatch");
		assert_eq!(
			request.headers.get(AUTHORIZATION).unwrap(),
			&"Bearer: l0tsofl3tters",
			"Header mismatch"
		);
		assert_eq!(request.body.unwrap(), b"yeet the thing", "Body mismatch");
	}
}
