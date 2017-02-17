//! Interface to the flatfile document storage database.

use std::path::PathBuf;
use std::fs::create_dir_all;

use futures::sync::mpsc::Sender;
use tantivy::schema::*;
use tantivy::collector::TopCollector;
use tantivy::query::QueryParser;
use tantivy::query::Query;

use tickgrinder_utils::conf::CONF;

pub struct ServerState {
  query_tx: Sender<String>,
}

/// Called to initialize the document store.  If the directory for document storage does not already exist, is is created
/// and if it does exist, it is indexed.
pub fn init_store() -> Result<ServerState, String> {
  // the directory in which the documents are stored
  let mut data_dir = PathBuf::from(CONF.data_dir);
  data_dir.push("documents");

  // create the directory if it doesn't exist
  create_dir_all(data_dir);


}
