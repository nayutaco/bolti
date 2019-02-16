use clap::{App, Arg};
use log::info;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};

fn main() -> io::Result<()> {
  // cargo build --release disables verbose log
  #[cfg(debug_assertions)]
  let _ = SimpleLogger::init(LevelFilter::Trace, Config::default());
  #[cfg(not(debug_assertions))]
  let _ = SimpleLogger::init(LevelFilter::Info, Config::default());

  let options = App::new("bolti")
    .about("send/receive BOLT messages interactively")
    .arg(
      Arg::with_name("port")
        .short("p")
        .long("port")
        .takes_value(true)
        .help("Set a TCP port to listen. If not specified, bolti will use some unused port."),
    )
    .get_matches();

  let port: u16 = options
    .value_of("port")
    .unwrap_or("0")
    .parse()
    .expect("port must be digits");

  let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))?;

  info!(
    "Waiting for incoming connection for {}...",
    listener.local_addr().unwrap()
  );

  for conn in listener.incoming() {
    let conn = conn?;
    info!("Connected by {}", conn.peer_addr().unwrap());
    handle_connection(conn)?;
  }

  Ok(())
}

fn handle_connection(conn: TcpStream) -> io::Result<()> {
  Ok(())
}
