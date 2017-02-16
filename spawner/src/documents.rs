//! Interface to the flatfile document storage database.

use std::path::PathBuf;
use std::fs::create_dir_all;
use std::thread;
use std::io::Error;

use futures::Stream;
use futures::sync::mpsc::{channel, Sender, Receiver};
use tantivy::schema::*;
use tantivy::collector::TopCollector;
use tantivy::query::QueryParser;
use tantivy::query::Query;

use tickgrinder_util::conf::CONF;

pub struct EngineHandle {
  /// The `Sender` used to submit work to the search engine
  query_tx: Sender<String>,
  /// The `Receiver` used to receive results from the search engine
  res_rx: Receiver<String>,
}

/// Called to initialize the document store.  If the directory for document storage does not already exist, is is created
/// and if it does exist, it is indexed.
pub fn init() -> Result<EngineHandle, Error> {
  // the directory in which the documents are stored
  let mut data_dir = PathBuf::from(CONF.data_dir);
  data_dir.push("documents");
  data_dir.push("tantivy_index");

  // create the directory if it doesn't exist
  try!(create_dir_all(data_dir));

  // create a channel to communicate with the server
  let (query_tx, query_rx) = channel(3);
  let res_rx = init_server_thread(query_rx);

  Ok(EngineHandle {
    query_tx: query_tx,
    res_rx: res_rx,
  })
}

/// Initializes main event loop that the server listens on.  Takes in queries via the provided `Receiver` and returns responses
/// through the returned `Receiver`.
pub fn init_server_thread(query_rx: Receiver<String>) -> Receiver<String> {
  let (res_tx, res_rx) = channel(3);

  thread::spawn(move || {
    for msg in query_rx.wait() {
      // TODO
    }
  });

  res_rx
}
