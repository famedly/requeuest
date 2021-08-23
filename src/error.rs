//! Errors specific to this crate.

use tokio::sync::oneshot::error::RecvError;

/// Errors which can happen in the job runner.
#[derive(Debug, Clone, Copy)]
pub enum JobError {
	/// No request was provided to the job, meaning no request can be sent.
	MissingRequest,
	/// A returning job could not find the sender for its job id to send the
	/// response through.
	MissingSender,
	/// The receiver for a returning job got dropped before the response could
	/// be sent.
	MissingReceiver,
}

impl std::fmt::Display for JobError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			JobError::MissingRequest => write!(f, "No request was provided for the job"),
			JobError::MissingSender => write!(f, "Job returning response couldn't find its sender"),
			JobError::MissingReceiver => {
				write!(f, "Receiver got dropped before the jobs response could be sent")
			}
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
	/// A request failed to (de)serialize
	Serde(bincode::Error),
}

impl std::error::Error for SpawnError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match *self {
			SpawnError::Sqlx(ref e) => Some(e),
			SpawnError::Receive(ref e) => Some(e),
			SpawnError::Serde(ref e) => Some(e),
		}
	}
}

impl std::fmt::Display for SpawnError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SpawnError::Receive(e) => write!(f, "Receiver error: {}", e),
			SpawnError::Sqlx(e) => write!(f, "SQL error: {}", e),
			SpawnError::Serde(e) => write!(f, "Serialization error: {}", e),
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

impl From<bincode::Error> for SpawnError {
	fn from(e: bincode::Error) -> Self {
		SpawnError::Serde(e)
	}
}

/// Errors which happen when converting requests from the [`http`] crate.
#[cfg(feature = "http")]
#[derive(Debug)]
pub enum ConvertError {
	/// A [`http::Request`] was incorrectly constructed
	Http(http::Error),
	/// The [`Uri`](http::Uri) of a request could not be converted to a
	/// [`Url`](url::Url).
	Url(url::ParseError),
}

#[cfg(feature = "http")]
impl std::error::Error for ConvertError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match *self {
			ConvertError::Http(ref e) => Some(e),
			ConvertError::Url(ref e) => Some(e),
		}
	}
}

#[cfg(feature = "http")]
impl std::fmt::Display for ConvertError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ConvertError::Http(e) => write!(f, "Bad http request: {}", e),
			ConvertError::Url(e) => write!(f, "URL parsing error: {}", e),
		}
	}
}

#[cfg(feature = "http")]
impl From<http::Error> for ConvertError {
	fn from(e: http::Error) -> Self {
		ConvertError::Http(e)
	}
}

#[cfg(feature = "http")]
impl From<url::ParseError> for ConvertError {
	fn from(e: url::ParseError) -> Self {
		ConvertError::Url(e)
	}
}
