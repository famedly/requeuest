//! Requeuest is a library for queueing the sending of HTTP requests. It's built
//! with the [sqlxmq](https://docs.rs/sqlxmq) crate, which is a message queue that uses a postgres database
//! for storing messages.
//!
//! ## Getting started
//! Assuming you already have an `sqlx` connection to a postgres database, you
//! will first need to run migrations so the needed tables and SQL functions can
//! get set up on your postgres database.
//!
//! ```no_run
//! # async fn test(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
//! requeuest::migrate(&pool).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Once that's taken care of, start by constructing a client. This is what you
//! will use to spawn requests, an what will execute jobs in the background. It
//! will keep doing so until it is dropped. The client contains a tokio
//! `JoinHandle` which you can remove from the client with
//! [`Client::take_listener`](crate::Client::take_listener) if you want the
//! listener to keep running after the client has dropped, or otherwise
//! interface with the background task directly.
//!
//! ```no_run
//! # async fn test(pool: sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
//! use requeuest::{Client, client::Channels};
//!
//! let client = Client::new(pool, Channels::List(&["my_service"])).await?;
//! # Ok(())
//! # }
//! ```
//!
//! After the client has been constructed, you can begin spawning jobs. Here we
//! send a get request to an example address:
//!
//! ```no_run
//! use requeuest::{HeaderMap, Request};
//!
//! # async fn test(client: requeuest::Client) -> Result<(), Box<dyn std::error::Error>> {
//! let request = Request::get("https://foo.bar/_api/baz".parse()?, HeaderMap::new());
//! client.spawn("my_service", &request).await?;
//! # Ok(())
//! # }
//! ```
//!
//! You can also also get the response back from a successfully delivered
//! request.
//!
//! ```no_run
//! # use requeuest::Request;
//!
//! # async fn test(client: requeuest::Client) -> Result<(), Box<dyn std::error::Error>> {
//! // You can skip the HeaderMap import by invoking the constructor via the Default trait
//! let request = Request::post("https://example.com/_api/bar/foo".parse()?, Vec::from("some data"), Default::default());
//! let response = client.spawn_returning("my_service", &request).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Note that the `spawn_returning` method *will* wait indefinitely (or to be
//! precise, roughly 10^293 years) until a successful response is received, so
//! this will wait forever if a request is sent to e.g. an unregistered domain,
//! or sends data to an API which will always result in a non-200 response code.
//!
//! # Features
//! This crate has the following features:
//! * `http`: Enable conversion of requests from the [`http`] crate
//! * Async runtime and TLS implementation for [`sqlx`]:
//!     * Any of `runtime-{tokio,actix,async-std}-{rustls,native-tls}`

#![doc(
	html_logo_url = "https://gitlab.com/famedly/company/backend/libraries/requeuest/-/raw/main/logo.svg"
)]
#![deny(missing_docs)]

pub mod client;
pub mod error;
pub(crate) mod job;
pub mod request;

pub use client::Client;
pub use request::Request;
pub use reqwest::{self, header::HeaderMap, Method};
use sqlx::{Pool, Postgres};
pub use url::Url;
pub use uuid::Uuid;

/// Runs the SQL migrations this library needs.
pub async fn migrate(pool: &Pool<Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
	sqlx::migrate!().run(pool).await
}
