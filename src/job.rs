//! Contains the definition of the job which sends http requests.

use crate::{error::JobError, request::Request};

use std::{collections::HashMap, sync::Mutex};

use sqlxmq::{job, CurrentJob};
use tokio::sync::{oneshot, OnceCell};
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

/// The function which runs HTTP jobs and actually sends the requests.
#[job(name = "http")]
pub async fn http(mut job: CurrentJob) -> JobResult {
	// validate the job payload
	let payload = job.raw_bytes().ok_or(JobError::MissingRequest)?;
	let request: Request = bincode::deserialize(payload)?;

	// construct and send the request
	let client = reqwest::Client::new();
	let mut builder = client.request(request.method, request.url);
	if let Some(body) = request.body {
		builder = builder.body(body);
	}
	let response = builder.send().await?;

	// complete the job if request was successful
	if request
		.accept_responses
		.contains(&response.status().as_u16())
	{
		job.complete().await?;
	}

	Ok(())
}

/// Sends the response to the HTTP request back via a oneshot channel.
#[job(name = "http_response")]
pub async fn http_response(mut job: CurrentJob) -> JobResult {
	// validate the job payload
	let payload = job.raw_bytes().ok_or(JobError::MissingRequest)?;
	let request: Request = bincode::deserialize(payload)?;

	// construct and send the request
	let client = reqwest::Client::new();
	let mut builder = client.request(request.method, request.url);
	if let Some(body) = request.body {
		builder = builder.body(body);
	}
	let response = builder.send().await?;

	// complete the job if request was successful
	if request
		.accept_responses
		.contains(&response.status().as_u16())
	{
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
