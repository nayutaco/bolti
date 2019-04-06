use lazy_static::lazy_static;
use lightning::ln::msgs;
use lightning::util::ser::Readable;
use std::collections::HashMap;
use std::io::Cursor;

lazy_static! {
  pub(crate) static ref deserializers: HashMap<u16, fn(&[u8]) -> String> = {
    let mut hashmap = HashMap::new();
    hashmap.insert(16, init as fn(&[u8]) -> String);
    hashmap
  };
}

pub(crate) fn init(bytes: &[u8]) -> String {
  let mut reader = Cursor::new(bytes);
  match msgs::Init::read(&mut reader) {
    Ok(init) => format!(
      "
init:
  supports_data_loss_protect: {}
  requires_data_loss_protect: {}
  initial_routing_sync: {}
  supports_upfront_shutdown_script: {}
  requires_upfront_shutdown_script: {}",
      init.local_features.supports_data_loss_protect(),
      init.local_features.requires_data_loss_protect(),
      init.local_features.initial_routing_sync(),
      init.local_features.supports_upfront_shutdown_script(),
      init.local_features.requires_upfront_shutdown_script()
    ),
    Err(err) => format!("bolti could not deserialize init: reason={:?}", err),
  }
}
