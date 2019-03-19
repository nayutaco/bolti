// rust-bindgen writes out .rs files that doesn't follow style conventions... ugh
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use clap::{App, Arg};
use log::{debug, info};
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::io;
use std::io::prelude::*;
use std::io::Read;
use std::mem::{uninitialized, zeroed};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::ptr::null_mut;
use std::slice::from_raw_parts;
use std::time::Duration;

type NodeId = Vec<u8>;

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

  unsafe { btc_rng_init() };
  generate_node_id();

  for conn in listener.incoming() {
    let mut conn = conn?;
    info!("Connected by {}", conn.peer_addr().unwrap());
    handle_connection(&mut conn)?;
  }

  Ok(())
}

fn generate_node_id() {
  unsafe {
    let mut keys: btc_keys_t = uninitialized();
    let succeed = btc_keys_create(&mut keys as *mut _);

    if !succeed {
      panic!("btc_keys_create failed!");
    }

    // TODO: dirty... needs refactoring.
    let node_id_ptr = ln_node_getid();
    (node_id_ptr as *mut [u8; 33]).write(keys.pub_);
    (node_id_ptr as *mut [u8; 32]).sub(1).write(keys.priv_);

    info!("My node id: {}", hex::encode(keys.pub_.to_vec()));
    debug!("My priv key: {}", hex::encode(keys.priv_));
  }
}

fn handle_connection(conn: &mut TcpStream) -> io::Result<()> {
  let mut p_channel: ln_channel_t = unsafe { uninitialized() };
  let node_id = noise_handshake(conn, &mut p_channel);

  let mut header: [u8; 18] = [0; 18];
  conn.set_read_timeout(None);  // blocks indefinitely
  let num_bytes = conn.read(&mut header).expect("Connection closed");
  assert_eq!(num_bytes, header.len());
  let decrypted_len: u16 = unsafe { ln_noise_dec_len(&mut p_channel.noise, header.as_ptr(), header.len() as u16) };

  let mut payload: Vec<u8> = vec![0; decrypted_len as usize];
  let num_bytes = conn.read(payload.as_mut_slice()).expect("Connection closed");
  assert_eq!(num_bytes, decrypted_len as usize);

  let mut buf: utl_buf_t = unsafe { zeroed() };
  unsafe { utl_buf_alloccopy(&mut buf as *mut utl_buf_t, payload.as_ptr(), payload.len() as u32) };
  let decryption_succeed = unsafe { ln_noise_dec_msg(&mut p_channel.noise, &mut buf) };
  if !decryption_succeed {
    panic!("DECODE: loop end");
  }
  debug!("{:?}", hex::encode(payload));

  Ok(())
}

fn noise_handshake(conn: &mut TcpStream, p_channel: &mut ln_channel_t) -> () {
  conn.set_read_timeout(Some(Duration::from_millis(10000))); // M_WAIT_RESPONSE_MSEC

  let mut b_cont = false;

  // Act 1
  let mut buf: utl_buf_t = unsafe { zeroed() };
  let act1_start_succeed = unsafe {
    ln_handshake_start(
      p_channel as *mut ln_channel_t,
      &mut buf as *mut utl_buf_t,
      null_mut(), // null indicates starting handshake as responder
    )
  };
  if !act1_start_succeed {
    panic!("fail: ln_handshake_start");
  }

  let mut rbuf: [u8; 50] = [0; 50];
  let num_bytes = conn.read(&mut rbuf).expect("Read timeout");
  if 0 >= num_bytes {
    panic!("Connection closed");
  }

  let act1_recv_succeed = unsafe {
    utl_buf_alloccopy(&mut buf as *mut utl_buf_t, rbuf.as_ptr(), rbuf.len() as u32);
    ln_handshake_recv(
      p_channel as *mut ln_channel_t,
      &mut b_cont as *mut bool,
      &mut buf as *mut utl_buf_t,
    )
  };

  if !act1_recv_succeed || !b_cont {
    panic!("fail: ln_handshake_recv1");
  }

  // Act 2
  conn.write(unsafe { from_raw_parts(buf.buf, buf.len as usize) }).expect("Act 2 failed");

  // Act 3
  let mut rbuf: [u8; 66] = [0; 66];
  let num_bytes = conn.read(&mut rbuf).expect("Read timeout");
  if 0 >= num_bytes {
    panic!("Connection closed")
  }

  let mut buf: utl_buf_t = unsafe { zeroed() };
  let act3_recv_succeed = unsafe {
    utl_buf_alloccopy(&mut buf as *mut utl_buf_t, rbuf.as_ptr(), rbuf.len() as u32);
    ln_handshake_recv(
      p_channel as *mut ln_channel_t,
      &mut b_cont as *mut bool,
      &mut buf as *mut utl_buf_t,
    )
  };
  if !act3_recv_succeed || b_cont {
    panic!("fail: ln_handshake_recv2");
  }

  assert_eq!(buf.len, 33);  // BTC_SZ_PUBKEY

  let opponent_node_id = unsafe { from_raw_parts(buf.buf, buf.len as usize) };
  info!("Handshake succeed! node id of the opponent is {}", hex::encode(opponent_node_id));

  ()
}
