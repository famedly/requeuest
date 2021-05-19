//! Errors specific to this crate.

/// Errors which can happen in the job runner.
#[derive(Debug, Clone, Copy)]
pub enum JobError {
	/// No request was provided to the job, meaning no request can be sent.
	MissingRequest,
}

impl std::fmt::Display for JobError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			JobError::MissingRequest => write!(f, "No request was provided for the job"),
		}
	}
}

impl std::error::Error for JobError {}

// TODO: define error type for spawning jobs.
