use ffi::Event;
use poll::Poll;
use std::env;
use std::{
    io::{self, Read, Result, Write},
    net::TcpStream,
};
//use std::{thread, time};

mod ffi;
mod poll;

fn get_req(path: &str) -> Vec<u8> {
    let s = format!(
        "GET {path} HTTP/1.1\r\n\
        Host: localhost\r\n\
        Connection: close\r\n\
        \r\n"
    );
    s.into()
}

fn handle_events(events: &[Event], streams: &mut [TcpStream]) -> Result<usize> {
    let mut handled_events = 0;
    for event in events {
        let index = event.token();
        let mut buffer = vec![0u8; 4096];

        loop {
            match streams[index].read(&mut buffer) {
                Ok(n) if n == 0 => {
                    handled_events += 1;
                    break;
                }
                Ok(n) => {
                    let tx = String::from_utf8_lossy(&buffer[..n]);
                    println!("Received: {:?}", event);
                    println!("{tx}\n---\n");
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(e) => return Err(e),
            }
        }
    }
    Ok(handled_events)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <port number>", args[0]);
        std::process::exit(1);
    }
    let port_number: u16 = args[1].parse().expect("Not a valid port number");

    let mut poll = Poll::new()?;
    let events_n = 3;
    let mut streams = vec![];
    let addr = format!("localhost:{port_number}");
    for i in 0..events_n {
        let delay_ms = (events_n - i) * 1000;
        let url_path = format!("/{delay_ms}/request-{i}");
        let request = get_req(&url_path);
        println!("About to connect");
        let mut stream = std::net::TcpStream::connect(&addr)?;
        println!("'Connected'");
        stream.set_nonblocking(true)?;
        stream.write_all(request.as_slice())?;
        poll.registry()
            .register(&stream, i, ffi::EPOLLIN | ffi::EPOLLET)?;
        streams.push(stream);
    }
    let mut handled_events = 0;
    while handled_events < events_n {
        let mut events = Vec::with_capacity(10);
        poll.poll(&mut events, None)?;
        if events.is_empty() {
            println!("Timeout or spur wakeup");
            continue;
        }
        handled_events += handle_events(&events, &mut streams)?;
    }
    //thread::sleep(time::Duration::from_millis(1000));
    Ok(())
}
