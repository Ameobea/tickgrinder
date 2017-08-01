//! Interface to the flatfile document storage database.

use std::path::PathBuf;
use std::fs::{create_dir_all, remove_file, read_dir, File, DirEntry};
use std::thread;
use std::fmt::Debug;
use std::io::{Read, ErrorKind};
use std::io::prelude::*;

use futures::Stream;
use futures::stream::MergedItem;
use futures::sync::mpsc::{channel, unbounded, Sender, Receiver, UnboundedSender, UnboundedReceiver};
use futures::sync::oneshot::Sender as Complete;
use tantivy::schema::{Schema, SchemaBuilder, Field, Value, TEXT, STORED, STRING};
use tantivy::collector::TopCollector;
use tantivy::query::{Query, QueryParser};
use tantivy::{Index, IndexWriter, Document};
use Uuid;
use serde_json::{to_string, to_string_pretty, from_str};

use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::commands::{Response, SrcDocument};
use tickgrinder_util::conf::CONF;

/// The type of query to run on the database.
#[derive(Copy, Clone)]
pub enum QueryType {
    BasicMatch, // returns documents that contain a single word
    TitleMatch, // returns documents that match the title only
}

/// Contains senders and receivers for interacting with the document store
#[derive(Clone)]
pub struct StoreHandle {
    /// The `Sender` used to submit work to the search engine.  Contains the query as a `String and a `Complete`
    /// to be fulfilled when the query has finished
    pub query_tx: UnboundedSender<(String, QueryType, Complete<Response>)>,
    /// The `Sender` used to add new documents to the index.  Contains the document as a `String` and a `Complete`
    /// to be fulfilled when the insertion has finished.
    pub insertion_tx: Option<Sender<(String, Complete<Response>)>>,
}

impl StoreHandle {
    /// Attempts to return the document with the specified title from the store.
    pub fn get_doc_by_title(&mut self, title: String, c: Complete<Response>) {
        let query_type = QueryType::TitleMatch;
        let query_tx = &self.query_tx;
        query_tx.send((title, query_type, c)).unwrap();
    }
}

/// Contains all of the `Field` objects for the different fields of the `Schema`
struct StoreFields {
    title: Field,
    body: Field,
    tags: Field,
    creation_date: Field,
    modification_date: Field,
    id: Field,
}

/// Utility function used to map errors of generic type to Strings by debug-formatting them
fn debug_err<T: Debug>(x: T) -> String {
    format!("{:?}", x)
}

/// Called to initialize the document store.  If the directory for document storage does not already exist, is is created
/// and if it does exist, it is indexed.
pub fn init_store_handle() -> Result<StoreHandle, String> {
    let mut cs = CommandServer::new(Uuid::new_v4(), "Tantivy Store Server");
    // the directory in which the documents are stored
    let mut data_dir = PathBuf::from(CONF.data_dir);
    data_dir.push("documents");
    data_dir.push("tantivy_index");

    // set up the index if it doesn't already exist and if it does, load it
    let (is_new, index) = if !data_dir.is_dir() {
        // create the directory if it doesn't exist
        cs.notice(None, &format!("Creating the document store directory at {:?}", data_dir));
        try!(create_dir_all(data_dir.clone()).map_err(debug_err));
        let schema = get_schema();
        (true, try!(Index::create(&data_dir, schema).map_err(debug_err)))
    } else {
        // delete the lock file if it exists from last time
        let mut lock_file_path = data_dir.clone();
        cs.notice(None, &format!("Deleting old lock file at {:?}", lock_file_path));
        lock_file_path.push(".tantivy-indexer.lock");
        match remove_file(lock_file_path) {
            Ok(_) => (),
            Err(err) => {
                match err.kind() {
                    ErrorKind::NotFound => {
                        // No file there so no issue we can't delete it
                        cs.notice(None, "Not deleting lock file because it doesn't exist.");
                    },
                    ErrorKind::PermissionDenied => {
                        cs.error(
                            None,
                            &format!(
                                "Permissions error while deleting lockfile from Tantivy store: {}; {}{}{}",
                                err,
                                "Make sure that the user running the Spawner has permissions to modify the ",
                                CONF.data_dir,
                                " directory."
                            )
                        );
                    },
                    _ => {
                        cs.error(
                            None,
                            &format!("Unhandled error while deleting lockfile from Tantivy store: {}", err)
                        );
                    }
                }
            }
        }

        // set up the flatfile storage directory if it doesn't exist
        let mut flatfile_dir = PathBuf::from(CONF.data_dir);
        flatfile_dir.push("documents");
        flatfile_dir.push("user_documents");
        if !flatfile_dir.is_dir() {
            try!(create_dir_all(flatfile_dir).map_err(debug_err));
        }

        // load the index from the directory
        cs.notice(None, &format!("Loading the index stored at {:?}", data_dir));
        (false, try!(Index::open(&data_dir).map_err(|err| format!("{:?}", err))))
    };

    // create channels to communicate with the server
    let (query_tx, query_rx) = unbounded();
    let (insertion_tx, insertion_rx) = channel(3);

    // start the server on another thread and start it listening for queries and insertion requests on the input channels
    init_server_thread(query_rx, insertion_rx, index, cs, is_new);

    Ok(StoreHandle {
        query_tx: query_tx,
        insertion_tx: Some(insertion_tx),
    })
}

/// Initializes main event loop that the server listens on.  Takes in queries via the provided `Receiver`s and returns responses
/// through the returned `Receiver`.
fn init_server_thread (
        query_rx: UnboundedReceiver<(String, QueryType, Complete<Response>)>, insertion_rx: Receiver<(String, Complete<Response>)>, index: Index,
        mut cs: CommandServer, is_new: bool
    ) {
    // merge the query and inserion streams so that they can be processed together in the event loop
    let merged_rx = query_rx.merge(insertion_rx);

    thread::spawn(move || {
        // set up some objects for use in accessing the store
        let schema = get_schema();
        let mut index_writer = index.writer(CONF.store_buffer_size).expect("Unable to create index writer!");

        if is_new {
            // load all the reference documents into the index if it's a new store
            load_static_docs(&mut cs, &mut index_writer);
        }

        for msg in merged_rx.wait() {
            let res: Result<(), Response> = (|| -> Result<(), Response> {
                match msg.expect("Msg was err in store event loop") {
                    MergedItem::First((query, query_type, complete)) => {
                        // execute the query and send the results through the `res_tx`
                        do_exec_query(query, query_type, complete, &index, &schema, &mut cs)
                    },
                    MergedItem::Second((doc_string, complete)) => {
                        // insert the document into the store
                        do_insert_doc(doc_string, complete, &mut index_writer, &mut cs)
                    },
                    MergedItem::Both((query, query_type, q_complete), (doc_string, i_complete)) => {
                        // both execute the query and insert a document into the store
                        do_exec_query(query, query_type, q_complete, &index, &schema, &mut cs)?;
                        do_insert_doc(doc_string, i_complete, &mut index_writer, &mut cs)
                    },
                }
            })();

            if res.is_err() {
                cs.error(
                    None,
                    &format!("There was an error processing a message from the document server's queue.  Response: {:?}", res.unwrap_err())
                );
            }
        }

        let hmm: Result<(), ()> = Ok(());
        hmm
    });
}

/// Inserts the document into the store and fulfills the oneshot once it's finished.
#[inline(always)]
fn do_insert_doc(doc_string: String, complete: Complete<Response>, index_writer: &mut IndexWriter, cs: &mut CommandServer) -> Result<(), Response> {
    match insert_document(&doc_string, index_writer, cs, true) {
        Ok(_) => {
            // let the client know the document was successfully inserted
            complete.send(Response::Ok)
        },
        Err(err) => {
            cs.error(None, &format!("Error while inserting document into store: {}", err));
            complete.send(Response::Error{status: format!("Unable to insert document into the store: {}", err)})
        },
    }
}

/// Executes the query in the store and handles the result.  Depending on the `QueryType`, the response is either a list of
/// matched titles in the case of a general query or the complete matched document if it was a title match.
#[inline(always)]
fn do_exec_query(
    query: String, query_type: QueryType, complete: Complete<Response>, index: &Index, schema: &Schema, cs: &mut CommandServer
) -> Result<(), Response> {
    let query_result = query_document(&query, query_type, index, &schema, 50);
    match query_result {
        Ok(res_vec) => {
            // send response to client by completing the oneshot
            match query_type {
                QueryType::TitleMatch => {
                    let res = if res_vec.len() > 0 {
                        Response::Document{
                            doc: from_str(&res_vec[0]).expect("Unable to parse stored document into `SrcDocument`"),
                        }
                    } else {
                        Response::Error{
                            status: format!("No documents matched the title {}", query),
                        }
                    };
                    complete.send(res)
                },
                _ => {
                    complete.send(Response::DocumentQueryResult{
                        results: res_vec,
                    })
                }
            }
        },
        Err(err) => {
            let errmsg = format!("Got error while executing query: {}", err);
            cs.error(None, &errmsg);
            complete.send(Response::Error{status: errmsg})
        },
    }
}

fn get_fields(schema: &Schema) -> StoreFields {
    StoreFields {
        title: schema.get_field("title").unwrap(),
        body: schema.get_field("body").unwrap(),
        tags: schema.get_field("tags").unwrap(),
        creation_date: schema.get_field("create-date").unwrap(),
        modification_date: schema.get_field("modify-date").unwrap(),
        id: schema.get_field("id").unwrap()
    }
}

/// Loads all of the JSON-encoded reference and user-created documents in the `documents/reference' and 'documents/user_documents'
/// directories into the Tantivy index.
fn load_static_docs(cs: &mut CommandServer, index_writer: &mut IndexWriter) {
    // iterator over the static documentation
    let mut reference_dir_path = PathBuf::from(CONF.data_dir);
    reference_dir_path.push("documents");
    reference_dir_path.push("reference");
    let ref_iterator = match read_dir(reference_dir_path) {
        Ok(iter) => iter,
        Err(err) => {
            // if we can't load the reference for some reason, oh well too bad.
            cs.error(None, &format!("Unable to create iterator over documentaton directory: {}", err));
            return;
        }
    };

    // iterator over the user-created documents
    let mut user_dir_path = PathBuf::from(CONF.data_dir);
    user_dir_path.push("documents");
    user_dir_path.push("user_documents");
    let user_iterator = match read_dir(user_dir_path) {
        Ok(iter) => iter,
        Err(err) => {
            // if we can't load the reference for some reason, oh well too bad.
            cs.error(None, &format!("Unable to create iterator over documentaton directory: {}", err));
            return;
        }
    };

    // iterate over all documents in both of the directories
    for doc_res in ref_iterator.chain(user_iterator) {
        let doc = match doc_res {
            Ok(f) => f,
            Err(ref err) => {
                cs.error(None, &format!("Got error reading file from documenation directory: {:?}", err));
                continue;
            },
        };

        // ignore any readme files or non-JSON files that may be laying around there
        match doc.path().extension() {
            Some(os_xt) => {
                if os_xt.to_str().expect("Unable to convert os_str to string").to_lowercase() == "json" {
                    // read the document into a `String` and convert to a `SrcDocument`
                    let doc_string: String = match load_doc_file(cs, &doc) {
                        Ok(d) => d,
                        Err(_) => continue,
                    };

                    // insert the document into the index
                    match insert_document(&doc_string, index_writer, cs, false) {
                        Ok(_) => {
                            cs.debug(
                                None,
                                &format!("Successfully inserted reference doc into store: {:?}", doc.path())
                            );
                        },
                        Err(err) => {
                            cs.error(
                                None,
                                &format!("Unable to insert reference doc into store: {:?}", err)
                            );
                        }
                    }
                }
            }
            None => (),
        }
    }

    cs.notice(None, "Finished loading all reference documents into index");
}

/// Given a `DirEntry`, attempts to read it into a `String`
fn load_doc_file(cs: &mut CommandServer, doc: &DirEntry) -> Result<String, ()> {
    let mut file = match File::open(&doc.path()) {
        Err(err) => {
            cs.error(None, &format!("Error reading file {:?}: {}", &doc.path(), err));
            return Err(());
        },
        Ok(file) => file,
    };

    let mut doc_contents = String::new();
    match file.read_to_string(&mut doc_contents) {
        Err(err) => {
            cs.error(None, &format!("Error reading file into string {:?}, {}", &doc.path(), err));
            return Err(());
        },
        Ok(_) => (),
    }

    Ok(doc_contents)
}

/// Inserts a document into the store and commits the changes to disk
fn insert_document(doc_str: &str, mut index_writer: &mut IndexWriter, cs: &mut CommandServer, is_new: bool) -> Result<u64, String> {
    let src_doc: SrcDocument = match from_str(doc_str) {
        Ok(doc) => doc,
        Err(err) => {
            return Err(format!("Unable to parse string into `SrcDocument`: {}, {}", doc_str, err))
        },
    };

    // write the document to a JSON flatfile if it's newly created
    if is_new {
        let mut doc_path = PathBuf::from(CONF.data_dir);
        doc_path.push("documents");
        doc_path.push("user_documents");
        doc_path.push(src_doc.id.hyphenated().to_string());
        doc_path.set_extension("json");
        match File::create(&doc_path) {
            Err(err) => {
                cs.error(None, &format!("Unable to create flatfile to store document: {:?}", err));
            }
            Ok(mut file) => {
                // Encode the `SrcDocument` into pretty-printed JSON and write it all into the file
                match to_string_pretty(&src_doc) {
                    Ok(json) => {
                        match file.write_all(json.as_bytes()) {
                            Ok(_) => cs.notice(None, &format!("Successfully wrote document to JSON file: {:?}", doc_path)),
                            Err(err) => cs.error(None, &format!("Unable to write document to JSON file: {:?}", err)),
                        }
                    },
                    Err(err) => cs.error(None, &format!("Unable to convert `SrcDocument` into pretty-printed JSON: {:?}", err)),
                };
            },
        }
    }

    let schema = get_schema();
    let StoreFields {title, body, tags, creation_date, modification_date, id} = get_fields(&schema);

    let mut doc = Document::default();
    doc.add_text(title, &src_doc.title);
    doc.add_text(body, &src_doc.body);
    // just join all the tags into a space-separated string like "tag1 tag2 tag3"
    let tags_string = src_doc.tags.join(" ");
    doc.add_text(tags, &tags_string);
    doc.add_text(creation_date, &src_doc.creation_date);
    doc.add_text(modification_date, &src_doc.modification_date);
    doc.add_text(id, &src_doc.id.hyphenated().to_string());

    // add the document to the store and commit the changes to disk
    index_writer.add_document(doc);
    index_writer.commit().map_err(debug_err)
}

/// Executes a query against the store, returning all matched documents.
fn query_document(
    raw_query: &str, query_type: QueryType, index: &Index, schema: &Schema, n_results: usize
) -> Result<Vec<String>, String> {
    let StoreFields {title, body, tags, creation_date, modification_date, id} = get_fields(&schema);
    let searcher = index.searcher();

    // convert the string-based query into a `Query` object and run the query
    let query: Box<Query> = match query_type {
        QueryType::BasicMatch => {
            let mut query_parser = QueryParser::new(index.schema(), vec!(title, body, tags));
            query_parser.set_conjunction_by_default();
            try!(query_parser.parse_query(raw_query.trim()).map_err(debug_err))
        },
        QueryType::TitleMatch => {
            let mut query_parser = QueryParser::new(index.schema(), vec!(title));
            query_parser.set_conjunction_by_default();
            try!(query_parser.parse_query(raw_query.trim()).map_err(debug_err))
        },
    };
    let mut top_collector = TopCollector::with_limit(n_results);
    try!(query.search(&searcher, &mut top_collector).map_err(debug_err));

    // collect the matched documents and return them as JSON
    let doc_addresses = top_collector.docs();
    let mut results = Vec::new();

    let val_to_string = |v: &Value| -> String {
        match v {
            &Value::Str(ref s) => s.clone(), // not the most efficient but it's the easiest and this isn't very hot code
            _ => panic!("Got a u32 value but expected a String"),
        }
    };

    for doc_address in doc_addresses {
        let retrieved_doc = try!(searcher.doc(&doc_address).map_err(debug_err));
        // convert into a SrcDocument
        let src_doc = SrcDocument {
            title: val_to_string(retrieved_doc.get_first(title).unwrap()),
            body: val_to_string(retrieved_doc.get_first(body).unwrap()),
            tags: val_to_string(retrieved_doc.get_first(tags).unwrap()).split_whitespace().map(|s| String::from(s)).collect(),
            creation_date: val_to_string(retrieved_doc.get_first(creation_date).unwrap()),
            modification_date: val_to_string(retrieved_doc.get_first(modification_date).unwrap()),
            id: Uuid::parse_str(&val_to_string(retrieved_doc.get_first(id).unwrap())).expect("Unable to parse document id into UUID!"),
        };
        // convert that SrcDocument into JSON and push it into the results
        results.push(to_string(&src_doc).expect("Unable to convert our own `SrcDocument` to `String`"));
    }

    Ok(results)
}

/// Returns a `Schema` object describing the schema of the document store.
fn get_schema() -> Schema {
    let mut schema_builder = SchemaBuilder::default();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT | STORED);
    schema_builder.add_text_field("tags", TEXT | STORED);
    schema_builder.add_text_field("create-date", STRING | STORED);
    schema_builder.add_text_field("modify-date", STRING | STORED);
    schema_builder.add_text_field("id", STRING | STORED);

    schema_builder.build()
}
