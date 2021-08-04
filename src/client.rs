//! The `Client` holds the job listener and database connection, which is used
//! to spawn jobs.

use std::borrow::Cow;

use sqlx::PgPool;
use sqlxmq::{JobBuilder, JobRegistry};
use uuid::Uuid;

use crate::{error::SpawnError, job, request::Request};

/// The list of channels the client should listen on
pub enum Channels<'a> {
	/// Listen on all channels requeuests are created for
	All,
	/// List of specific channels requeuest should listen on
	List(&'a [&'a str]),
}

/// The client is used for listening for and spawning new jobs.
#[derive(Debug)]
pub struct Client {
	/// The database connection pool.
	pool: PgPool,
	/// The handle to the tokio task which listens for and spawns jobs in the
	/// background.
	listener: Option<sqlxmq::OwnedHandle>,
}

impl Client {
	/// Constructs a new client, which listens for jobs on the given channels.
	///
	/// It will stop running jobs when it goes out of scope, unless
	/// `take_listener` listener is called.
	pub async fn new(pool: PgPool, channels: Channels<'_>) -> Result<Self, sqlx::Error> {
		let mut registry = JobRegistry::new(&[job::http, job::http_response]);
		registry.set_context(reqwest::Client::new());

		let mut listener = registry.runner(&pool);
		if let Channels::List(channels) = channels {
			listener.set_channel_names(channels);
		}

		Ok(Self { pool, listener: Some(listener.run().await?) })
	}

	/// Takes the tokio `JoinHandle` which listens for and runs spawned jobs,
	/// and prevents it from being aborted when the client is dropped. Returns
	/// `None` if the handle has already been taken.
	pub fn take_listener(&mut self) -> Option<tokio::task::JoinHandle<()>> {
		if let Some(handle) = self.listener.take() {
			return Some(handle.into_inner());
		}
		None
	}

	/// Returns true if the handle to the listener has been taken out of the
	/// client with the `take_listener` method.
	pub fn is_detached(&self) -> bool {
		self.listener.is_none()
	}

	/// Get a reference to the client's database connection.
	pub fn pool(&self) -> &PgPool {
		&self.pool
	}

	/// Spawns a request on the given channel. Returns the UUID of the spawned
	/// job.
	///
	/// # Example
	/// ```no_run
	/// # async fn test(pool: sqlx::postgres::PgPool) -> Result<(), Box<dyn std::error::Error>> {
	/// use requeuest::{Client, Request, client::Channels};
	///
	/// let client = Client::new(pool, Channels::List(&["my_service"])).await?;
	/// let request = Request::get("https://foo.bar/baz".parse()?, Default::default());
	/// client.spawn("my_service", &request).await?;
	/// # Ok(())
	/// # }
	pub async fn spawn<'a, C: Into<Cow<'static, str>>>(
		&'a self,
		channel: C,
		request: &'a Request,
	) -> Result<Uuid, SpawnError> {
		request.spawn_with(&self.pool, channel).await
	}

	/// Spawn a job. Accepts a closure which lets you set custom job
	/// parameters, such as if a job should be ordered and how many retry
	/// attempts should be made. See [`sqlxmq::JobBuilder`](sqlxmq::JobBuilder)
	/// for available configurations.
	///
	/// # Example
	/// ```no_run
	/// # use requeuest::{Client, Request, error::SpawnError};
	/// # async fn example(client: Client, request: Request) -> Result<(), SpawnError> {
	/// client.spawn_cfg("my_app", &request, |job| { job.set_ordered(false); }).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn spawn_cfg<'a, C: Into<Cow<'static, str>>>(
		&'a self,
		channel: C,
		request: &'a Request,
		cfg: impl for<'b> FnOnce(&'b mut JobBuilder),
	) -> Result<Uuid, SpawnError> {
		request.spawn_with_cfg(&self.pool, channel, cfg).await
	}

	/// Spawns a request and awaits until a response with an accepted status
	/// code has been received, returning the received response. This method
	/// will wait indefinitely until a succressful response has been received,
	/// so be careful that your request is correctly constructed, and that you
	/// don't inadvertently hang your program when calling this ethod.
	pub async fn spawn_returning<'a, C: Into<Cow<'static, str>>>(
		&'a self,
		channel: C,
		request: &'a Request,
	) -> Result<reqwest::Response, SpawnError> {
		request.spawn_returning_with(&self.pool, channel).await
	}

	/// Spawn a returning job. Accetps a closure which lets you set custom job
	/// parameters. See [`sqlxmq::JobBuilder`](sqlxmq::JobBuilder) for available
	/// configurations.
	pub async fn spawn_returning_cfg<'a, C: Into<Cow<'static, str>>>(
		&'a self,
		channel: C,
		request: &'a Request,
		cfg: impl for<'b> FnOnce(&'b mut JobBuilder),
	) -> Result<reqwest::Response, SpawnError> {
		request.spawn_returning_with_cfg(&self.pool, channel, cfg).await
	}
}
