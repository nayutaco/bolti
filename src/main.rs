// rust-bindgen writes out .rs files that doesn't follow style conventions... ugh
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use clap::{App, Arg};
use log::{debug, info, warn};
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::io;
use std::io::prelude::*;
use std::io::{Read, Cursor};
use std::mem::{uninitialized, zeroed};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::ptr::null_mut;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::time::Duration;
use std::ops::{Deref, DerefMut};
use lightning::ln::msgs::Init;
use lightning::util::ser::Readable;
use lazy_static::lazy_static;
mod deser;

struct UtlBuf(utl_buf_t);

impl UtlBuf {
  fn new() -> Self {
    Self(unsafe { zeroed() })
  }

  fn as_mut_ptr(&mut self) -> *mut utl_buf_t {
    &mut self.0 as *mut utl_buf_t
  }

  fn as_slice(&self) -> &[u8] {
    unsafe { from_raw_parts(self.0.buf, self.0.len as usize) }
  }
}

impl Deref for UtlBuf {
  type Target = utl_buf_t;

  fn deref(&self) -> &utl_buf_t {
    &self.0
  }
}

impl DerefMut for UtlBuf {
  fn deref_mut(&mut self) -> &mut utl_buf_t {
    &mut self.0
  }
}

impl Drop for UtlBuf {
  fn drop(&mut self) {
    unsafe {
      utl_buf_free(self.as_mut_ptr())
    }
  }
}

impl<T: AsRef<[u8]>> From<T> for UtlBuf {
  fn from(seq: T) -> Self {
    let slice: &[u8] = seq.as_ref();
    let mut buf = Self::new();
    unsafe {
      utl_buf_alloccopy(buf.as_mut_ptr(), slice.as_ptr(), slice.len() as u32);  // Why not usize?
      buf
    }
  }
}

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
    handle_connection(&mut conn);
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

fn handle_connection(conn: &mut TcpStream) {
  let mut p_channel: ln_channel_t = unsafe { uninitialized() };
  let node_id = noise_handshake(conn, &mut p_channel);

  let mut header: [u8; 18] = [0; 18];
  conn.set_read_timeout(None).expect("Something went wrong");  // blocks indefinitely

  loop {
    let num_bytes = conn.read(&mut header).expect("Connection closed");
    assert_eq!(num_bytes, header.len());
    let decrypted_len: u16 = unsafe { ln_noise_dec_len(&mut p_channel.noise, header.as_ptr(), header.len() as u16) };

    let mut payload: Vec<u8> = vec![0; decrypted_len as usize];
    let num_bytes = conn.read(payload.as_mut_slice()).expect("Connection closed");
    assert_eq!(num_bytes, decrypted_len as usize);

    let mut buf: UtlBuf = payload.into();
    let decryption_succeed = unsafe { ln_noise_dec_msg(&mut p_channel.noise, &mut buf.0) };
    if !decryption_succeed {
      panic!("DECODE: loop end");
    }
    // info!("Received: {}", hex::encode(buf.as_slice()));

    let mut type_slice: [u8; 2] = [0; 2];
    type_slice.copy_from_slice(&buf.as_slice()[..2]);
    let type_num = u16::from_be_bytes(type_slice);
    match deser::deserializers.get(&type_num) {
      Some(deserializer) => info!("{}", deserializer(&buf.as_slice()[2..])),
      None => warn!("Unknown message arrived: type={}", type_num)
    }
  }
}

fn noise_handshake(conn: &mut TcpStream, p_channel: &mut ln_channel_t) -> () {
  conn.set_read_timeout(Some(Duration::from_millis(10000))).expect("Handshake failed due to timeout"); // M_WAIT_RESPONSE_MSEC

  let mut b_cont = false;

  // Act 1
  let mut buf = UtlBuf::new();
  let act1_start_succeed = unsafe {
    ln_handshake_start(
      p_channel as *mut ln_channel_t,
      buf.as_mut_ptr(),
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

  let mut buf: UtlBuf = rbuf[..].into();
  let act1_recv_succeed = unsafe {
    ln_handshake_recv(
      p_channel as *mut ln_channel_t,
      &mut b_cont as *mut bool,
      buf.as_mut_ptr(),
    )
  };

  if !act1_recv_succeed || !b_cont {
    panic!("fail: ln_handshake_recv1");
  }

  // Act 2
  conn.write(buf.as_slice()).expect("Act 2 failed");

  // Act 3
  let mut rbuf: [u8; 66] = [0; 66];
  let num_bytes = conn.read(&mut rbuf).expect("Read timeout");
  if 0 >= num_bytes {
    panic!("Connection closed")
  }

  let mut buf: UtlBuf = rbuf[..].into();
  let act3_recv_succeed = unsafe {
    ln_handshake_recv(
      p_channel as *mut ln_channel_t,
      &mut b_cont as *mut bool,
      buf.as_mut_ptr(),
    )
  };
  if !act3_recv_succeed || b_cont {
    panic!("fail: ln_handshake_recv2");
  }

  assert_eq!(buf.len, 33);  // BTC_SZ_PUBKEY

  let opponent_node_id = buf.as_slice();
  info!("Handshake succeed! node id of the opponent is {}", hex::encode(opponent_node_id));

  ()
}
