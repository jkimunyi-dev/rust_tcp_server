use std::collections::HashMap;
use std::fmt::Display;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
};
use std::thread;
use std::time::{Duration, SystemTime};

const SAFE_MODE: bool = true;
const BAN_LIMIT: Duration = Duration::from_secs(10 * 60);

type Result<T> = std::result::Result<T, ()>;

struct Client {
    conn: Arc<TcpStream>,
    last_message: SystemTime,
    strike_count: i64,
}

enum Message {
    ClientConnected {
        author: Arc<TcpStream>,
    },
    ClientDisconnected {
        author: Arc<TcpStream>,
    },
    NewMessage {
        author: Arc<TcpStream>,
        bytes: Vec<u8>,
    },
}

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
        .send(Message::ClientConnected {
            author: stream.clone(),
        })
        .map_err(|err| eprintln!("ERROR :  Could not send mesasge to the server thread : {err}"))?;

    let mut buffer = [0; 512];

    loop {
        match stream.as_ref().read(&mut buffer) {
            Ok(0) => {
                let _ = messages
                    .send(Message::ClientDisconnected {
                        author: stream.clone(),
                    })
                    .map_err(|err| {
                        eprintln!("ERROR : Could not send disconnected message : {err}");
                    });
                return Ok(());
            }

            Ok(n) => {
                let _ = messages
                    .send(Message::NewMessage {
                        author: stream.clone(),
                        bytes: buffer[0..n].to_vec(),
                    })
                    .map_err(|err| {
                        eprintln!("ERROR : Could not send message to the server thread : {err}")
                    });
            }

            Err(_) => {
                let _ = messages
                    .send(Message::ClientDisconnected {
                        author: stream.clone(),
                    })
                    .map_err(|err| {
                        eprintln!("ERROR : Could not send disconnected message : {err}")
                    });

                return Ok(());
            }
        }
    }
}

fn server(messages: Receiver<Message>) -> Result<()> {
    let mut clients: HashMap<String, Client> = HashMap::new();

    let mut banned_mfs: HashMap<String, SystemTime> = HashMap::new();

    let now = SystemTime::now();

    loop {
        let msg = messages
            .recv()
            .map_err(|err| eprintln!("ERROR : Server reciever failed : {err}"))?;

        match msg {
            Message::ClientConnected { author } => {
                if let Ok(addr) = author.peer_addr() {
                    let addr_str = addr.to_string();

                    let mut banned_at = banned_mfs.remove(&addr.ip().to_string());

                    banned_at = banned_at.and_then(|banned_at| {
                        let diff = now
                            .duration_since(banned_at)
                            .expect("Handle time may have gone backwards");

                        if diff >= BAN_LIMIT {
                            None
                        } else {
                            Some(banned_at)
                        }
                    });
                    println!("Client connected : {}", Sensitive(&addr_str));

                    if let Some(banned_at) = banned_at {
                        let diff = now
                            .duration_since(banned_at)
                            .expect("Handle time may have gone backwards");

                        banned_mfs.insert(addr.ip().to_string(), banned_at);
                        let mut author = author.as_ref();
                        writeln!(
                            author,
                            "You are banned : {} secs left",
                            (BAN_LIMIT - diff).as_secs_f32()
                        );

                        let _ = author.shutdown(Shutdown::Both);
                    } else {
                        clients.insert(
                            addr_str,
                            Client {
                                conn: author.clone(),
                                last_message: SystemTime::now(),
                                strike_count: 0,
                            },
                        );
                    }
                }
            }
            Message::ClientDisconnected { author } => {
                if let Ok(addr) = author.peer_addr() {
                    let addr_str = addr.to_string();

                    println!("Client Disconnected : {}", Sensitive(&addr_str));
                    clients.remove(&addr_str);
                }
            }
            Message::NewMessage { author, bytes } => {
                if let Ok(author_addr) = author.peer_addr() {
                    let author_addr_str = author_addr.to_string();

                    // Vector of Clients to Avoid Borrow Checker Issues
                    let clients_to_send: Vec<_> = clients
                        .iter()
                        .filter(|(addr, _)| **addr != author_addr_str)
                        .map(|(_, client)| client.conn.clone())
                        .collect();

                    // Send Message to other Clients

                    for client_conn in clients_to_send {
                        if let Err(err) = client_conn.as_ref().write_all(&bytes) {
                            eprintln!("Falied to Write to client : {err}");
                        }
                    }
                }
            }
        }
    }
    // Ok(())
}

fn main() -> Result<()> {
    let address = "127.0.0.1:6969";

    println!("INFO : Listening to {}", Sensitive(address));

    let (message_sender, message_reciever) = channel();

    thread::spawn(move || {
        if let Err(err) = server(message_reciever) {
            eprintln!("ERROR : Server Thread failed : {:?}", err);
        }
    });

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
