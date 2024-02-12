use hyper::{service::service_fn, body::Incoming};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use hyper_util::{
    rt::{TokioIo, TokioExecutor},
    server::conn::auto};
use std::convert::Infallible;
use hyper::{Response, Request};
use hyper::body::Bytes;
use http_body_util::Full;
use std::error::Error;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;
async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

async fn proxy_handler(mut req: Request<hyper::body::Incoming>) ->  Result<Response<BoxBody>, Box<dyn Error + Send + Sync>> {

    
    let uri_string = format!(
    "http://{}{}",
    "127.0.0.1:3000".to_string(),
    req.uri()
    .path_and_query()
    .map(|x| x.as_str())
    .unwrap_or("/"));

    let uri = uri_string.parse().unwrap();
    *req.uri_mut() = uri;

    let host = req.uri().host().expect("uri has no host");
    let port = req.uri().port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);
    
    let client_stream = TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(client_stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

    let response = sender.send_request(req).await?;

    Ok(Response::new(BoxBody::new(response.into_body())))
}

async fn serve(in_addr: SocketAddr, out_addr:SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    
    let listener = TcpListener::bind(in_addr).await?;

    println!("Listening on http://{}", in_addr);
    println!("Proxying on http://{}", out_addr);

    let _out_addr_clone = out_addr.clone();
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        // This is the `Service` that will handle the connection.
        // `service_fn` is a helper to convert a function that
        // returns a Response into a `Service`.

        tokio::task::spawn(async move {
            if let Err(err) = auto::Builder::new(TokioExecutor::new()).serve_connection(io, service_fn(proxy_handler)).await {
                println!("Failed to serve the connection: {:?}", err);
            }
        });
    }
}

#[tokio::main]
async fn main() {

    let in_addr: SocketAddr = ([127, 0, 0, 1], 3001).into();
    let out_addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

    let _ = serve(in_addr, out_addr).await;


}
