//! Contains the definition of the job which sends http requests.

use std::{
	collections::HashMap,
	sync::{Arc, LockResult, Mutex, MutexGuard},
};

use sqlxmq::{job, CurrentJob};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{error::JobError, request::Request};

/// Alias for the result type sqlxmq jobs expect.
pub type JobResult = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

/// Alias for a map from request UUID to associated oneshot sender
type SenderMap = HashMap<Uuid, oneshot::Sender<reqwest::Response>>;

/// Mechanism for returning responses from successful jobs.
#[derive(Debug)]
pub(crate) struct ResponseSender(Arc<Mutex<SenderMap>>);

impl ResponseSender {
	/// Constructs a new response sender.
	pub fn new() -> ResponseSender {
		ResponseSender(Arc::new(Mutex::new(HashMap::new())))
	}

	/// Gets a lock to the mutex wrapping the map.
	pub fn lock(&self) -> LockResult<MutexGuard<'_, SenderMap>> {
		self.0.lock()
	}
}

impl Clone for ResponseSender {
	fn clone(&self) -> Self {
		ResponseSender(Arc::clone(&self.0))
	}
}

/// The function which runs HTTP jobs and actually sends the requests.
#[job(name = "http")]
pub async fn http(mut job: CurrentJob, client: reqwest::Client) -> JobResult {
	// validate the job payload
	let payload = job.raw_bytes().ok_or(JobError::MissingRequest)?;
	let request: Request = bincode::deserialize(payload)?;

	// construct and send the request
	let mut builder = client.request(request.method, request.url).headers(request.headers);
	if let Some(body) = request.body {
		builder = builder.body(body);
	}
	let response = builder.send().await?;

	// complete the job if the response is in the accepted set
	if request.accept_responses.iter().any(|accepted| accepted.accepts(response.status())) {
		job.complete().await?;
	}

	Ok(())
}

/// Sends the response to the HTTP request back via a oneshot channel.
#[job(name = "http_response")]
pub async fn http_response(
	mut job: CurrentJob,
	client: reqwest::Client,
	sender: ResponseSender,
) -> JobResult {
	// validate the job payload
	let payload = job.raw_bytes().ok_or(JobError::MissingRequest)?;
	let request: Request = bincode::deserialize(payload)?;

	// construct and send the request
	let mut builder = client.request(request.method, request.url);
	if let Some(body) = request.body {
		builder = builder.body(body);
	}
	let response = builder.send().await?;

	// complete the job if the response is in the accepted set
	if request.accept_responses.iter().any(|accepted| accepted.accepts(response.status())) {
		job.complete().await?;

		#[allow(clippy::unwrap_used)] // We don't handle poisoning
		let sender = sender.0.lock().unwrap().remove(&job.id()).ok_or(JobError::MissingSender)?;
		sender.send(response).or(Err(JobError::MissingReceiver))?;
	}

	Ok(())
}
