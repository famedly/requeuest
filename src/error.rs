//! Errors specific to this crate.

use tokio::sync::oneshot::error::RecvError;

/// Errors which can happen in the job runner.
#[derive(Debug, Clone, Copy)]
pub enum JobError {
	/// No request was provided to the job, meaning no request can be sent.
	MissingRequest,
	/// A returning job could not find the sender for its job id to send the response through.
	MissingSender,
	/// The receiver for a returning job got dropped before the response could be sent.
	MissingReceiver,
}

impl std::fmt::Display for JobError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			JobError::MissingRequest => write!(f, "No request was provided for the job"),
			JobError::MissingSender => write!(f, "Job returning response couldn't find its sender"),
			JobError::MissingReceiver => write!(
				f,
				"Receiver got dropped before the jobs response could be sent"
			),
		}
	}
}

impl std::error::Error for JobError {}

/// An error that can occur when spawning a job.
#[derive(Debug)]
pub enum SpawnError {
	/// The sql query was not successfully executed
	Sqlx(sqlx::Error),
	/// The response sender was dropped before the response could be received.
	Receive(RecvError),
}

impl std::error::Error for SpawnError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match *self {
			SpawnError::Sqlx(ref e) => Some(e),
			SpawnError::Receive(ref e) => Some(e),
		}
	}
}

impl std::fmt::Display for SpawnError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SpawnError::Receive(e) => write!(f, "Receiver error: {}", e),
			SpawnError::Sqlx(e) => write!(f, "SQL error: {}", e),
		}
	}
}

impl From<sqlx::Error> for SpawnError {
	fn from(e: sqlx::Error) -> Self {
		SpawnError::Sqlx(e)
	}
}

impl From<RecvError> for SpawnError {
	fn from(e: RecvError) -> Self {
		SpawnError::Receive(e)
	}
}
