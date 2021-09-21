//! Tests that verify that HTTP requeuests are correctly sent

use std::{
	iter::FromIterator,
	sync::atomic::{AtomicU32, Ordering},
	time::Duration,
};

use requeuest::{
	self,
	client::{Channels, Client},
	request::Request,
	HeaderMap, Url,
};
use reqwest::header::{HeaderValue, AUTHORIZATION};
use tokio::sync::Notify;

static INSTALL_EYRE: std::sync::Once = std::sync::Once::new();

fn install_eyre() {
	INSTALL_EYRE.call_once(|| color_eyre::install().expect("Installing eyre failed"))
}

/// Crate a hyper `Service` with the given future as its `service_fn`
#[macro_export]
macro_rules! service {
	($closure:expr) => {
		hyper::service::make_service_fn(|_conn| async {
			Ok::<_, hyper::Error>(hyper::service::service_fn($closure))
		})
	};
}

/// Create a hyper `Server` with the given graceful shutdown closure, returning
/// the ip address and server object.
#[macro_export]
macro_rules! server {
	($service:expr, $shutdown:expr) => {{
		let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 0));
		let server = hyper::Server::bind(&addr).serve($service);
		// Get the address from the server so we know what port was picked
		let addr = server.local_addr();
		let server = server.with_graceful_shutdown($shutdown);
		(addr, server)
	}};
}

static SEND_EMPTY_NOTIF: Notify = Notify::const_new();

/// Verifies that a request without a body is correctly sent
#[sqlx_database_tester::test(pool(variable = "pool", skip_migrations))]
#[ntest::timeout(60_000)]
async fn send_empty() -> color_eyre::eyre::Result<()> {
	install_eyre();
	requeuest::migrate(&pool).await?;
	let client = Client::new(pool, Channels::All).await?;

	let service = service!(|req| async move {
		assert_eq!(req.method(), hyper::Method::GET, "Wrong method");
		assert_eq!(req.uri().path(), "/path", "Wrong URI path");
		assert_eq!(req.uri().query().unwrap(), "query=foo&param=bar", "Wrong URI query");
		assert_eq!(req.headers()[AUTHORIZATION], &"Bearer: secret", "Wrong HTTP header");
		SEND_EMPTY_NOTIF.notify_one();
		Ok::<_, hyper::Error>(hyper::Response::new(hyper::Body::from("OK")))
	});

	let (addr, server) = server!(service, async { SEND_EMPTY_NOTIF.notified().await });

	let headers =
		HeaderMap::from_iter([(AUTHORIZATION, HeaderValue::from_static("Bearer: secret"))]);
	let request =
		Request::get(format!("http://{}/path?query=foo&param=bar", addr).parse()?, headers);

	client.spawn("channel", &request).await?;

	server.await?;

	Ok(())
}

static RETRY_COUNT: AtomicU32 = AtomicU32::new(0);
static RETRY_NOTIF: Notify = Notify::const_new();

/// Verifies that a request is correctly retried
#[sqlx_database_tester::test(pool(variable = "pool", skip_migrations))]
#[ntest::timeout(60_000)]
async fn retry() -> color_eyre::eyre::Result<()> {
	install_eyre();
	requeuest::migrate(&pool).await?;
	let client = Client::new(pool, Channels::All).await?;

	let service = service!(|_req| async move {
		let attempt = RETRY_COUNT.fetch_add(1, Ordering::SeqCst);
		let response = match attempt {
			0..=2 => {
				hyper::Response::builder().status(400).body(hyper::Body::from("Try again")).unwrap()
			}
			3 => {
				RETRY_NOTIF.notify_one();
				hyper::Response::new(hyper::Body::from("OK"))
			}
			_ => panic!("Too many retries!"),
		};
		Ok::<_, hyper::Error>(response)
	});

	let (addr, server) = server!(service, async { RETRY_NOTIF.notified().await });

	let request = Request::get(format!("http://{}/", addr).parse()?, Default::default());

	client
		.spawn_cfg("channel", &request, |req| {
			req.set_retries(3);
			req.set_retry_backoff(Duration::from_millis(10));
		})
		.await?;

	server.await?;

	Ok(())
}

static ORDER_NOTIF: Notify = Notify::const_new();
static ORDER_REQ_NUM: AtomicU32 = AtomicU32::new(1);

/// Verifies that returning requests get delivered sequentially
#[sqlx_database_tester::test(pool(variable = "pool", skip_migrations))]
#[ntest::timeout(30_000)]
async fn order() -> color_eyre::eyre::Result<()> {
	install_eyre();
	requeuest::migrate(&pool).await?;
	let client = Client::new(pool, Channels::All).await?;

	let service = service!(|req: hyper::Request<hyper::Body>| async move {
		let num = ORDER_REQ_NUM.fetch_add(1, Ordering::AcqRel);
		let body = hyper::body::to_bytes(req.into_body()).await?;
		let req_num = String::from_utf8_lossy(&body).parse::<u32>();

		if req_num != Ok(num) {
			panic!("Wrong order");
		}
		if num == 3 {
			ORDER_NOTIF.notify_one()
		}

		Ok::<_, hyper::Error>(hyper::Response::new(hyper::Body::from("OK")))
	});

	let (addr, server) = server!(service, async { ORDER_NOTIF.notified().await });

	let url: Url = format!("http://{}/", addr).parse()?;
	let request1 = Request::post(url.clone(), b"1".to_vec(), Default::default());
	let request2 = Request::post(url.clone(), b"2".to_vec(), Default::default());
	let request3 = Request::post(url, b"3".to_vec(), Default::default());

	let handle = tokio::spawn(async move { server.await });

	let cfg = |job: &mut sqlxmq::JobBuilder| {
		job.set_ordered(true);
	};
	client.spawn_cfg("order", &request1, cfg).await?;
	client.spawn_cfg("order", &request2, cfg).await?;
	client.spawn_cfg("order", &request3, cfg).await?;

	handle.await??;

	Ok(())
}
