# Famedly

**This project is part of the source code of Famedly.**

We think that software for healthcare should be open source, so we publish most 
parts of our source code at [gitlab.com/famedly](https://gitlab.com/famedly).

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of
conduct, and the process for submitting pull requests to us.

For licensing information of this project, have a look at the [LICENSE.md](LICENSE.md)
file within the repository.

If you compile the open source software that we make available to develop your
own mobile, desktop or embeddable application, and cause that application to
connect to our servers for any purposes, you have to aggree to our Terms of
Service. In short, if you choose to connect to our servers, certain restrictions
apply as follows:  

* You agree not to change the way the open source software connects and
interacts with our servers
* You agree not to weaken any of the security features of the open source software
* You agree not to use the open source software to gather data
* You agree not to use our servers to store data for purposes other than
the intended and original functionality of the Software
* You acknowledge that you are solely responsible for any and all updates to
your software

No license is granted to the Famedly trademark and its associated logos, all of
which will continue to be owned exclusively by Famedly GmbH. Any use of the
Famedly trademark and/or its associated logos is expressly prohibited without
the express prior written consent of Famedly GmbH.

For more
information take a look at [Famedly.com](https://famedly.com) or contact
us by [info@famedly.com](mailto:info@famedly.com?subject=[GitLab]%20More%20Information%20)

---

# Requeuest
Requeuest (pronounced "recused") is a message queue which acts as an intermediary for HTTP requests, making sure that the sent request gets successfully delivered eventually, meaning that you do not have to implement retry logic for HTTP API requests. The queue uses the [`sqlxmq`] crate to make postgres its store for messages, which avoids the reliability risk of a dedicated message queue service potentially being down. This comes with the trade-off that job runners become part of the library consumer's process, and that a handle to the runner has to be kept alive so jobs can run in the background, since jobs cannot be delegated to a separate runner service.

## Getting started
Assuming you already have an `sqlx` connection to a postgres database, you will first need to run migrations so the needed tables and SQL functions can get set up on your postgres database.
```rust
requeuest::migrate(&pool).await?;
```
Once that's taken care of, start by constructing a client. This is what you will use to spawn requests, an what will execute jobs in the background. It will keep doing so until it is dropped. The client contains a tokio `JoinHandle` which you can remove from the client with the `Client::take_listener` method if you want the listener to keep running after the client has dropped, or otherwise interface with the background task directly.
```rust
use requeuest::Client;

let client = Client::new(pool, &["my_service"]).await?;
```
After the client has been constructed, you can begin spawning jobs. Here we send a get request to an example address:
```rust
use requeuest::{HeaderMap, Request};

let request = Request::get("https://foo.bar/_api/baz".parse()?, HeaderMap::new());
client.spawn("my_service", &request).await?;
```
You can also also get the response back from a successfully delivered request.
```rust
// You can skip the HeaderMap import by invoking the constructor via the Default trait
let request = Request::post("https://example.com/_api/bar/foo".parse()?, Vec::from("some data"), Default::default());
let response = client.spawn_returning("my_service", &request).await?;
```
Note that the `spawn_returning` method *will* wait indefinitely (or to be precise, roughly 10^293 years) until a successful response is received, so this will wait forever if a request is sent to e.g. an unregistered domain, or a request to an API which that's guaranteed to always get a response back with a non-200 response code.

[`sqlxmq`]: https://docs.rs/sqlxmq
