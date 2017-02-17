//! Interface to the flatfile document storage database.

use std::path::{Path, PathBuf};
use std::fs::create_dir_all;
use std::thread;
use std::fmt::Debug;
use std::sync::Arc;

use futures::{Future, Sink, Stream};
use futures::stream::MergedItem;
use futures::sync::mpsc::{channel, Sender, Receiver};
use tantivy::schema::{Schema, SchemaBuilder, Field, TEXT, STORED, STRING};
use tantivy::collector::TopCollector;
use tantivy::query::QueryParser;
use tantivy::{Index, IndexWriter, Document};
use Uuid;
use serde_json::from_str;

use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::conf::CONF;

/// Contains senders and receivers for interacting with the document store
#[derive(Clone)]
pub struct StoreHandle {
    /// The `Sender` used to submit work to the search engine
    query_tx: Sender<String>,
    /// The `Sender` used to add new documents to the index
    insertion_tx: Sender<String>,
    /// The `Receiver` used to receive results from the search engine
    res_rx: Arc<Receiver<Vec<String>>>,
}

/// A document that can be stored in the database.
#[derive(Serialize, Deserialize)]
struct SrcDocument {
    title: String,
    body: String,
    tags: Vec<String>,
    creation_date: String,
    modification_date: String,
}

/// Contains all of the `Field` objects for the different fields of the `Schema`
struct StoreFields {
    title: Field,
    body: Field,
    tags: Field,
    creation_date: Field,
    modification_date: Field,
}

/// Utility function used to map errors of generic type to Strings by debug-formatting them
fn debug_err<T: Debug>(x: T) -> String {
    format!("{:?}", x)
}

/// Called to initialize the document store.  If the directory for document storage does not already exist, is is created
/// and if it does exist, it is indexed.
pub fn init_store_handle() -> Result<StoreHandle, String> {
    // the directory in which the documents are stored
    let mut data_dir = PathBuf::from(CONF.data_dir);
    data_dir.push("documents");
    data_dir.push("tantivy_index");

    let index = if !data_dir.is_dir() {
        // create the directory if it doesn't exist
        try!(create_dir_all(data_dir.clone()).map_err(debug_err));
        let schema = get_schema();
        try!(Index::create(&data_dir, schema).map_err(debug_err))
    } else {
        try!(Index::open(&data_dir).map_err(|err| format!("{:?}", err)))
    };

    // create channels to communicate with the server
    let (query_tx, query_rx) = channel(3);
    let (insertion_tx, insertion_rx) = channel(3);

    let res_rx = try!(init_server_thread(query_rx, insertion_rx, index));

    Ok(StoreHandle {
        query_tx: query_tx,
        insertion_tx: insertion_tx,
        res_rx: Arc::new(res_rx),
    })
}

/// Initializes main event loop that the server listens on.  Takes in queries via the provided `Receiver`s and returns responses
/// through the returned `Receiver`.
fn init_server_thread (
        query_rx: Receiver<String>, insertion_rx: Receiver<String>, index: Index
    ) -> Result<Receiver<Vec<String>>, String> {
    let (mut res_tx, res_rx) = channel::<Vec<String>>(3);
    let mut cs = CommandServer::new(Uuid::new_v4(), "Tantivy Store Server");

    // merge the query and inserion streams so that they can be processed together in the event loop
    let merged_rx = query_rx.merge(insertion_rx);

    thread::spawn(move || {
        // set up some objects for use in accessing the store
        let schema = get_schema();
        let mut index_writer = index.writer(CONF.store_buffer_size).unwrap();

        for msg in merged_rx.wait() {
            match msg.expect("Msg was err in store event loop") {
                MergedItem::First(query) => {
                    // execute the query and send the results through the `res_tx`
                    let query_result = query_document(&query, &index, &schema, 50);
                    match query_result {
                        Ok(res_vec) => {
                            res_tx = res_tx.send(res_vec).wait().unwrap();
                        },
                        Err(err) => {
                            cs.error(None, &format!("Got error while executing query: {}", err));
                        },
                    }
                },
                MergedItem::Second(doc_string) => {
                    // insert the document into the store
                    match insert_document(&doc_string, &mut index_writer) {
                        Ok(_) => (),
                        Err(err) => cs.error(None, &format!("Error while inserting document into store: {}", err)),
                    }
                },
                MergedItem::Both(query, doc_string) => {
                    // both execute the query and insert a document into the store
                    let query_result = query_document(&query, &index, &schema, 50);
                    match query_result {
                        Ok(res_vec) => {
                            res_tx = res_tx.send(res_vec).wait().unwrap();
                        },
                        Err(err) => {
                            cs.error(None, &format!("Got error while executing query: {}", err));
                        },
                    }

                    match insert_document(&doc_string, &mut index_writer) {
                        Ok(_) => (),
                        Err(err) => cs.error(None, &format!("Error while inserting document into store: {}", err)),
                    }
                },
            }
        }

        let hmm: Result<(), ()> = Ok(());
        return hmm
    });

    Ok(res_rx)
}

fn get_fields(schema: &Schema) -> StoreFields {
    StoreFields {
        title: schema.get_field("title").unwrap(),
        body: schema.get_field("body").unwrap(),
        tags: schema.get_field("tags").unwrap(),
        creation_date: schema.get_field("create-date").unwrap(),
        modification_date: schema.get_field("modify-date").unwrap()
    }
}

/// Inserts a document into the store and commits the changes to disk
fn insert_document(doc_str: &str, mut index_writer: &mut IndexWriter) -> Result<u64, String> {
    let src_doc: SrcDocument = match from_str(doc_str) {
        Ok(doc) => doc,
        Err(err) => {
            return Err(format!("Unable to parse string into `SrcDocument`: {}", doc_str))
        },
    };
    let schema = get_schema();
    let StoreFields {title, body, tags, creation_date, modification_date} = get_fields(&schema);

    let mut doc = Document::default();
    doc.add_text(title, &src_doc.title);
    doc.add_text(body, &src_doc.body);
    // just join all the tags into a space-separated string like "tag1 tag2 tag3"
    let tags_string = src_doc.tags.join(" ");
    doc.add_text(tags, &tags_string);
    doc.add_text(creation_date, &src_doc.creation_date);
    doc.add_text(modification_date, &src_doc.modification_date);

    // add the document to the store and commit the changes to disk
    try!(index_writer.add_document(doc).map_err(debug_err));
    index_writer.commit().map_err(debug_err)
}

/// Executes a query against the store, returning all matched documents.
fn query_document(raw_query: &str, index: &Index, schema: &Schema, n_results: usize) -> Result<Vec<String>, String> {
    let StoreFields {title, body, tags: _, creation_date: _, modification_date: _} = get_fields(&schema);
    let searcher = index.searcher();
    let query_parser = QueryParser::new(index.schema(), vec!(title, body));

    // convert the string-based query into a `Query` object and run the query
    let query = try!(query_parser.parse_query(raw_query).map_err(debug_err));
    let mut top_collector = TopCollector::with_limit(n_results);
    try!(query.search(&searcher, &mut top_collector).map_err(debug_err));

    // collect the matched documents and return them as JSON
    let doc_addresses = top_collector.docs();
    let mut results = Vec::new();
    for doc_address in doc_addresses {
        let retrieved_doc = try!(searcher.doc(&doc_address).map_err(debug_err));
        results.push(schema.to_json(&retrieved_doc));
    }

    Ok(results)
}

/// Returns a `Schema` object describing the schema of the document store.
fn get_schema() -> Schema {
    let mut schema_builder = SchemaBuilder::default();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("title", TEXT);
    schema_builder.add_text_field("tags",    TEXT);
    schema_builder.add_text_field("create-date", STRING);
    schema_builder.add_text_field("modify-data", STRING);

    schema_builder.build()
}

/// Exports all the indexed documents into JSON format.
fn export_documents(dst_dir: &Path) {
    unimplemented!(); // TODO
}

/// Imports all the JSON-encoded documents from the source directory and adds them to the store.
fn import_documents(src_dir: &Path) {
    unimplemented!(); // TODO
}
