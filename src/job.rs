//! Contains the definition of the job which sends http requests.

use crate::error::JobError;

use std::{collections::HashMap, sync::Mutex};

use reqwest::{header::HeaderMap, Method, StatusCode};
use serde::{Deserialize, Serialize};
use sqlxmq::{job, CurrentJob};
use tokio::sync::{oneshot, OnceCell};
use url::Url;
use uuid::Uuid;

/// Alias for the result type sqlxmq jobs expect.
pub type JobResult = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

type ResponseSender = Mutex<HashMap<Uuid, oneshot::Sender<reqwest::Response>>>;

static RESPONSE_SENDERS: OnceCell<ResponseSender> = OnceCell::const_new();

async fn senders_init() -> ResponseSender {
	Mutex::new(HashMap::new())
}

pub(crate) async fn response_senders<'a>() -> &'a ResponseSender {
	RESPONSE_SENDERS.get_or_init(senders_init).await
}

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

/// The function which runs HTTP jobs and actually sends the requests.
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

/// Sends the response to the HTTP request back via a oneshot channel.
#[job(name = "http_response")]
pub async fn http_response(mut job: CurrentJob) -> JobResult {
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

		let sender_map = response_senders().await;
		let sender = sender_map
			.lock()
			.unwrap()
			.remove(&job.id())
			.ok_or(JobError::MissingSender)?;
		sender.send(response).or(Err(JobError::MissingReceiver))?;
	}

	Ok(())
}
