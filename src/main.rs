use std::fmt::Display;
use std::io::Read;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

type Result<T> = std::result::Result<T, ()>;

enum Message {
    ClientConnected(Arc<TcpStream>),
    ClientDisconnected(Arc<TcpStream>),
    NewMessage(Vec<u8>),
}

const SAFE_MODE: bool = true;

#[derive(Debug)]
struct Sensitive<T>(T);

impl<T: Display> Display for Sensitive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(inner) = self;

        if SAFE_MODE {
            writeln!(f, "[REDACTED]")
        } else {
            inner.fmt(f)
        }
    }
}

fn client(stream: Arc<TcpStream>, messages: Sender<Message>) -> Result<()> {
    messages
        .send(Message::ClientConnected(stream.clone()))
        .map_err(|err| eprintln!("ERROR :  Could not send mesasge to the server thread : {err}"))?;

    let mut buffer = Vec::new();
    buffer.resize(512, 0);

    loop {
        let n = stream.as_ref().read(&mut buffer).map_err(|_err| {
            let _ = messages.send(Message::ClientDisconnected(stream.clone()));
            // ()_
        })?;

        let _ = messages
            .send(Message::NewMessage(buffer[0..n].to_vec()))
            .map_err(|err| eprintln!("ERROR : Could not send message to server thread : {err}"))?;
    }
}

fn server(_channel: Receiver<Message>) -> Result<()> {
    Ok(())
}

fn main() -> Result<()> {
    let address = "127.0.0.1:6969";

    println!("INFO : Listening to {}", Sensitive(address));

    let (message_sender, message_reciever) = channel();

    thread::spawn(|| server(message_reciever));

    let listener = TcpListener::bind(address)
        .map_err(|err| eprintln!("Could not bind {} : {}", Sensitive(address), Sensitive(err)))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let stream = Arc::new(stream);
                let message_sender = message_sender.clone();
                let _ = thread::spawn(|| client(stream, message_sender));
            }

            Err(err) => eprintln!("ERROR : Could not write data to user : {err}"),
        }
    }

    Ok(())
}
