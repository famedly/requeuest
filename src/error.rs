//! Errors specific to this crate.

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

// TODO: define error type for spawning jobs.
