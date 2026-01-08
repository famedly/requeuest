//! The `Client` holds the job listener and database connection, which is used
//! to spawn jobs.

use std::borrow::Cow;

use sqlx::PgPool;
use sqlxmq::{JobBuilder, JobRegistry, JobRunnerHandle};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{error::SpawnError, job, job::ResponseSender, request::Request};

/// Prototype function that applies default settings for sqlx jobs
fn default_job_proto<'a>(builder: &'a mut JobBuilder<'a>) -> &'a mut JobBuilder<'a> {
	builder.set_retries(100_000).set_ordered(true)
}

/// The list of channels the client should listen on
#[derive(Debug)]
pub enum Channels<'a> {
	/// Listen on all channels requeuests are created for
	All,
	/// List of specific channels requeuest should listen on
	List(&'a [&'a str]),
}

/// The client is used for listening for and spawning new jobs.
pub struct Client {
	/// The database connection pool.
	pool: PgPool,
	/// The handle to the runner which listens for and spawns jobs in the
	/// background.
	listener: Option<JobRunnerHandle>,
	/// A map of oneshot channels which successful responses are sent through.
	response_sender: ResponseSender,
}

impl std::fmt::Debug for Client {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Client").finish()
	}
}

impl Client {
	/// Constructs a new client, which listens for jobs on the given channels.
	///
	/// It will stop running jobs when it goes out of scope, unless
	/// `take_listener` listener is called.
	pub async fn new(pool: PgPool, channels: Channels<'_>) -> Result<Self, sqlx::Error> {
		let mut registry = JobRegistry::new(&[job::http, job::http_response]);
		let response_sender = ResponseSender::new();
		registry.set_context(reqwest::Client::new());
		registry.set_context(response_sender.clone());

		let mut listener = registry.runner(&pool);
		if let Channels::List(channels) = channels {
			listener.set_channel_names(channels);
		}

		Ok(Self { pool, listener: Some(listener.run().await?), response_sender })
	}

	/// Takes the runner handle which listens for and runs spawned jobs,
	/// and prevents it from being aborted when the client is dropped.
	/// Returns `None` if the handle has already been taken.
	pub fn take_listener(&mut self) -> Option<JobRunnerHandle> {
		self.listener.take()
	}

	/// Returns true if the handle to the listener has been taken out of the
	/// client with the `take_listener` method.
	#[must_use]
	pub fn is_detached(&self) -> bool {
		self.listener.is_none()
	}

	/// Get a reference to the client's database connection.
	#[must_use]
	pub fn pool(&self) -> &PgPool {
		&self.pool
	}

	/// Removes all pending jobs from the given set of channels.
	pub async fn clear(&self, channels: Channels<'_>) -> Result<(), sqlx::Error> {
		match channels {
			Channels::All => sqlxmq::clear_all(&self.pool).await,
			Channels::List(list) => sqlxmq::clear(&self.pool, list).await,
		}
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
	/// let request = Request::get("https://foo.bar/baz")?.build();
	/// client.spawn("my_service", &request).await?;
	/// # Ok(())
	/// # }
	pub async fn spawn<'a, C: Into<Cow<'static, str>> + Send>(
		&'a self,
		channel: C,
		request: &'a Request,
	) -> Result<Uuid, SpawnError> {
		let uuid = retrying_spawn(
			job::http
				.builder()
				.set_raw_bytes(&bincode::serialize(request)?)
				.set_channel_name(channel.into().as_ref())
				.set_proto(default_job_proto),
			&self.pool,
		)
		.await?;
		Ok(uuid)
	}

	/// Spawn a job. Accepts a closure which lets you set custom job
	/// parameters, such as  retry attempts should be made. By default jobs are
	/// retried 100 000 times. See [`sqlxmq::JobBuilder`](sqlxmq::JobBuilder)
	/// for available configurations. They include:
	/// * [Number of retries](sqlxmq::JobBuilder::set_retries)
	/// * [Initial retry backoff](sqlxmq::JobBuilder::set_retry_backoff)
	/// * <del>If the job is ordered</del> Ordering is currently disabled due to
	///   a bug in [`sqlxmq`].
	/// * [Delay before execution](sqlxmq::JobBuilder::set_delay)
	///
	/// # Example
	/// ```no_run
	/// # use requeuest::{Client, Request, error::SpawnError};
	/// # async fn example(client: Client, request: Request) -> Result<(), SpawnError> {
	/// client.spawn_cfg("my_app", &request, |job| { job.set_ordered(false); }).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn spawn_cfg<'a, C: Into<Cow<'static, str>> + Send>(
		&'a self,
		channel: C,
		request: &'a Request,
		cfg: impl for<'b> FnOnce(&'b mut JobBuilder) + Send,
	) -> Result<Uuid, SpawnError> {
		let mut builder = job::http.builder();

		let builder = builder.set_proto(default_job_proto);
		cfg(builder);
		let uuid = retrying_spawn(
			builder
				.set_channel_name(channel.into().as_ref())
				.set_raw_bytes(&bincode::serialize(request)?),
			&self.pool,
		)
		.await?;

		Ok(uuid)
	}

	/// Spawns a request and awaits until a response with an accepted status
	/// code has been received, returning the received response. This method
	/// will wait indefinitely until a succressful response has been received,
	/// so be careful that your request is correctly constructed, and that you
	/// don't inadvertently hang your program when calling this ethod.
	pub async fn spawn_returning<'a, C: Into<Cow<'static, str>> + Send>(
		&'a self,
		channel: C,
		request: &'a Request,
	) -> Result<reqwest::Response, SpawnError> {
		// Put a sender in the sender map so the job can use it
		let uuid = Uuid::new_v4();
		let (sender, receiver) = oneshot::channel();
		#[allow(clippy::unwrap_used)] // We don't handle poisoning
		self.response_sender.lock().unwrap().insert(uuid, sender);

		// Spawn the job
		retrying_spawn(
			job::http_response
				.builder_with_id(uuid)
				.set_raw_bytes(&bincode::serialize(request)?)
				.set_channel_name(channel.into().as_ref())
				.set_proto(default_job_proto),
			&self.pool,
		)
		.await?;

		Ok(receiver.await?)
	}

	/// Spawn a returning job. Accetps a closure which lets you set custom job
	/// parameters. See [`sqlxmq::JobBuilder`](sqlxmq::JobBuilder) for available
	/// configurations. They include:
	/// * [Number of retries](sqlxmq::JobBuilder::set_retries)
	/// * [Initial retry backoff](sqlxmq::JobBuilder::set_retry_backoff)
	/// * [If the job is ordered](sqlxmq::JobBuilder::set_ordered)
	/// * [Delay before execution](sqlxmq::JobBuilder::set_delay)
	pub async fn spawn_returning_cfg<'a, C: Into<Cow<'static, str>> + Send>(
		&'a self,
		channel: C,
		request: &'a Request,
		cfg: impl for<'b> FnOnce(&'b mut JobBuilder) + Send,
	) -> Result<reqwest::Response, SpawnError> {
		// Put a sender in the sender map so the job can use it
		let uuid = Uuid::new_v4();
		let (sender, receiver) = oneshot::channel();
		#[allow(clippy::unwrap_used)] // We don't handle poisoning
		self.response_sender.lock().unwrap().insert(uuid, sender);

		// Spawn the job
		let mut builder = job::http_response.builder_with_id(uuid);
		let builder = builder.set_proto(default_job_proto);
		cfg(builder);
		retrying_spawn(
			builder
				.set_raw_bytes(&bincode::serialize(request)?)
				.set_channel_name(channel.into().as_ref())
				.set_ordered(false),
			&self.pool,
		)
		.await?;
		Ok(receiver.await?)
	}
}

/// Retry spawning a job if we receive certain database errors.
async fn retrying_spawn<'a>(job: &'a JobBuilder<'a>, pool: &PgPool) -> Result<Uuid, SpawnError> {
	let uuid = loop {
		let result = job.spawn(pool).await;
		// Retry on constraint violations,
		match result {
			Err(e) if sqlxmq::should_retry(&e) => continue,
			Err(e) => return Err(e.into()),
			Ok(uuid) => break uuid,
		}
	};
	Ok(uuid)
}
