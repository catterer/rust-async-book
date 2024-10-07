#[macro_use] extern crate log;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::env;

use hyper::body::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use tokio::net::TcpListener;
use hyper_util::rt::{TokioIo, TokioTimer};
use regex::Regex;
use once_cell::sync::Lazy;
use tokio::time::{sleep, Duration};

static RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^/([0-9]+)/([^/]*)$").expect("Invalid regex")
});

fn resp500(err: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::new(bytes::Bytes::from(format!("500 internval server error: {err}"))))
        .unwrap()
}


// An async function that consumes a request, does nothing with it and returns a
// response.
async fn hello(req: Request<impl hyper::body::Body>) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();
    info!(">> {path}");
    if let Some(captures) = RE.captures(path) {
        let delay_ms_str = captures.get(1).map_or("", |m| m.as_str());
        let delay_ms: u64 = match delay_ms_str.parse() {
            Ok(ms) => ms,
            Err(_) => return Ok(resp500("invalid delay value"))
        };

        let req_id = captures.get(2).map_or("", |m| m.as_str());
        sleep(Duration::from_millis(delay_ms)).await;
        Ok(Response::new(bytes::Bytes::from(format!("{req_id}: delayed by {delay_ms} ms")).into()))
    } else {
        Ok(resp500("invalid path"))
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <port number>", args[0]);
        std::process::exit(1);
    }

    tracing_subscriber::fmt::init();
    trace!("OLOLO");

    let port_number: u16 = args[1].parse().expect("Not a valid port number");
    let addr: SocketAddr = ([127, 0, 0, 1], port_number).into();

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        // When an incoming TCP connection is received grab a TCP stream for
        // client<->server communication.
        //
        // Note, this is a .await point, this loop will loop forever but is not a busy loop. The
        // .await point allows the Tokio runtime to pull the task off of the thread until the task
        // has work to do. In this case, a connection arrives on the port we are listening on and
        // the task is woken up, at which point the task is then put back on a thread, and is
        // driven forward by the runtime, eventually yielding a TCP stream.
        let (tcp, _) = listener.accept().await?;
        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(tcp);

        // Spin up a new task in Tokio so we can continue to listen for new TCP connection on the
        // current task without waiting for the processing of the HTTP1 connection we just received
        // to finish
        tokio::task::spawn(async move {
            // Handle the connection from the client using HTTP1 and pass any
            // HTTP requests received on that connection to the `hello` function
            if let Err(err) = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, service_fn(hello))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
