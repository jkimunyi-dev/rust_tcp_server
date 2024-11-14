use std::fmt::Display;
use std::io::Write;
use std::{net::TcpListener, result};

type Result<T> = result::Result<(), T>;

const SAFE_MODE: bool = false;

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

fn main() -> Result<()> {
    let address = "127.0.0.1:6969";

    println!("INFO : Listening to {}", Sensitive(address));

    let listener = TcpListener::bind(address)
        .map_err(|err| eprintln!("Could not bind {} : {}", Sensitive(address), Sensitive(err)))?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = writeln!(stream, "Hello friend")
                    .map_err(|err| eprintln!("ERROR : Could not write message to user : {err}"));

                // println!("{:?}", Sensitive(stream))
            }

            Err(err) => eprintln!("ERROR : Could not write data to user : {err}"),
        }
    }

    Ok(())
}
