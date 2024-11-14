use std::fmt::Display;
use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::{net::TcpListener, result};

type Result<T> = result::Result<(), T>;

enum Message {
    _ClientConnected,
    _ClientDisconnected,
    _NewMessage,
}

const SAFE_MODE: bool = false;

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

fn client(mut stream: TcpStream, _messages: Sender<Message>) -> Result<()> {
    let _ = writeln!(stream, "Hello friend")
        .map_err(|err| eprintln!("ERROR : Could not write message to user : {err}"));

    println!("{:?}", Sensitive(stream));

    Ok(())
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
                let message_sender = message_sender.clone();
                let _ = thread::spawn(|| client(stream, message_sender));
            }

            Err(err) => eprintln!("ERROR : Could not write data to user : {err}"),
        }
    }

    Ok(())
}
