//! The `Client` holds the job listener and database connection, which is used to spawn jobs.

use crate::{error::SpawnError, job, request::Request};

use sqlx::PgPool;
use sqlxmq::JobRegistry;
use uuid::Uuid;

/// The client is used for listening for and spawning new jobs.
#[derive(Debug)]
pub struct Client {
	/// The database connection pool.
	pool: PgPool,
	/// The handle to the tokio task which listens for and spawns jobs in the background.
	listener: Option<sqlxmq::OwnedHandle>,
}

impl Client {
	/// Constructs a new client, which listens for jobs on the given channels. It will stop running
	/// jobs when it goes out of scope, unless `take_listener` listener is called.
	pub async fn new(pool: PgPool, channels: &[&str]) -> Result<Self, sqlx::Error> {
		let registry = JobRegistry::new(&[job::http, job::http_response]);
		let listener = registry
			.runner(&pool)
			.set_channel_names(channels)
			.run()
			.await?;
		Ok(Self {
			pool,
			listener: Some(listener),
		})
	}

	/// Takes the tokio `JoinHandle` which listens for and runs spawned jobs, and prevents it from
	/// being aborted when the client is dropped. Returns `None` if the handle has already been
	/// taken.
	pub fn take_listener(&mut self) -> Option<tokio::task::JoinHandle<()>> {
		if let Some(mut handle) = self.listener.take() {
			// We can't move the inner handle directly, since the newtype containing it implements
			// Drop, so we replace it instead.
			let inner = std::mem::replace(&mut handle.0, tokio::task::spawn(async {}));
			return Some(inner);
		}
		None
	}

	/// Returns true if the handle to the listener has been taken out of the client with the
	/// `take_listener` method.
	pub fn is_detached(&self) -> bool {
		self.listener.is_none()
	}

	/// Get a reference to the client's database connection.
	pub fn pool(&self) -> &PgPool {
		&self.pool
	}

	/// Spawns a request on the given channel. Returns the UUID of the spawned job.
	///
	/// # Example
	/// ```no_run
	/// # async fn test(pool: sqlx::postgres::PgPool) -> Result<(), Box<dyn std::error::Error>> {
	/// use requeuest::{Client, Request};
	///
	/// let client = Client::new(pool, &["my_service"]).await?;
	/// let request = Request::get("https://foo.bar/baz".parse()?, Default::default());
	/// client.spawn("my_service", &request).await?;
	/// # Ok(())
	/// # }
	pub async fn spawn<'a>(
		&'a self,
		channel: &'static str,
		request: &'a Request,
	) -> Result<Uuid, SpawnError> {
		let uuid = job::http
			.builder()
			.set_raw_bytes(&bincode::serialize(request)?)
			.set_channel_name(channel)
			.set_retries(100_000)
			.spawn(&self.pool)
			.await?;
		Ok(uuid)
	}

	/// Spawns a request and await until a response with a 200 status code has been received,
	/// returning the received response. This method will wait indefinitely until a succressful
	/// response has been received, so be careful that your request is correctly constructed, and
	/// that you don't inadverdently hang your program when calling this ethod.
	pub async fn spawn_returning<'a>(
		&'a self,
		channel: &'static str,
		request: &'a Request,
	) -> Result<reqwest::Response, SpawnError> {
		// Put a sender in the sender map so the job can use it
		let uuid = Uuid::new_v4();
		let (sender, receiver) = tokio::sync::oneshot::channel();
		job::response_senders()
			.await
			.lock()
			.unwrap()
			.insert(uuid, sender);

		// Spawn the job
		job::http_response
			.builder_with_id(uuid)
			.set_raw_bytes(&bincode::serialize(request)?)
			.set_channel_name(channel)
			.spawn(&self.pool)
			.await?;
		Ok(receiver.await?)
	}
}
