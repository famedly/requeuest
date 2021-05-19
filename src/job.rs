//! Contains the definition of the job which sends http requests.

use crate::error::JobError;

use reqwest::{header::HeaderMap, Method, StatusCode};
use serde::{Deserialize, Serialize};
use sqlxmq::{job, CurrentJob};
use url::Url;

/// Alias for the result type sqlxmq jobs expect.
pub type JobResult = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

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

#[job(name = "http")]
pub async fn http(mut job: CurrentJob) -> JobResult {
	// validate the job payload
	let request: Request = job.json()?.ok_or(JobError::MissingRequest)?;

	// construct and send the request
	let client = reqwest::Client::new();
	let mut builder = client.request(request.method, request.url);
	if let Some(body) = request.body {
		builder = builder.body(body);
	}
	let response = builder.send().await?;

	// complete the job if request was successful
	if response.status() == StatusCode::OK {
		job.complete().await?;
	}

	Ok(())
}
