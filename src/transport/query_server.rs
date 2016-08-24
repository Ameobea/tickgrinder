// Connection Pool-esque construct used to queue up and asynchronously execute
// Postgres queries ane optionally evaluate callbacks based on the results.

use std::collections::VecDeque;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

use postgres;
use futures::stream::{Stream, channel, Sender, Receiver};
use futures::Future;

use transport::postgres::get_client;

type QueryError = postgres::error::Error;
type SenderQueue = Arc<Mutex<VecDeque<Sender<String, ()>>>>;
type QueryQueue = Arc<Mutex<VecDeque<String>>>;

pub struct QueryServer {
    conn_count: usize, // how many connections to open
    query_queue: QueryQueue, // internal query queue
    conn_queue: SenderQueue, // Database connection objects
}

// locks the QueryQueue and returns a queued query, if there are any.
// DO NOT CALL THIS FROM THE MAIN THREAD; WILL BLOCK EVERYTHING
fn try_get_new_query(query_queue: QueryQueue) -> Option<String> {
    println!("Locking query_queue in worker thread");
    let mut qq_inner = query_queue.lock().unwrap();
    // there is a queued query
    if !qq_inner.is_empty() {
        return Some(qq_inner.pop_front().unwrap())
    }else{
        // No new queries
        return None
    }
}

fn execute_query(query: String, client: &postgres::Connection) {
    println!("Sending query: {:?}", query);
    println!("Current thread: {:?}", thread::current().name());
    client.execute(query.as_str(), &[]);
    thread::sleep(Duration::new(6, 0));
}

// Creates a query processor that awaits requests
fn init_query_processor(tx: Sender<Sender<String, ()>, ()>, query_queue: QueryQueue) {
    // get a connection to the postgres database
    println!("Worker process started on thread {:?}", thread::current().name());
    let client = get_client().expect("Couldn't create postgres connection.");
    // channel to receive queries from the main thread
    let (tx_tx, tx_rx) = channel::<String, ()>();
    // send back a Sender to the main thread that can be used to send queries to the worker
    tx.send(Ok(tx_tx)).forget();
    // handler for new queries from main thread
    // This BLOCKS the worker thread until a new message is received
    for query in tx_rx.wait() {
        println!("Thread inside the receiver callback: {:?}", thread::current().name());
        execute_query(query.unwrap(), &client);
        while let Some(new_query) = try_get_new_query(query_queue.clone()) {
            execute_query(new_query, &client);
        }
    }
}

impl QueryServer {
    pub fn new(conn_count: usize) -> QueryServer {
        let mut conn_queue = VecDeque::with_capacity(conn_count);
        let query_queue = Arc::new(Mutex::new(VecDeque::new()));
        for i in 0..conn_count {
            // channel for getting the Sender back from the worker thread
            let (tx, rx) = channel::<Sender<String, ()>, ()>();
            let qq_copy = query_queue.clone();
            let handle = thread::Builder::new().name(format!("worker{}", i))
                .spawn(move || { init_query_processor(tx, qq_copy) });
            println!("Spawned thread with name {:?}", handle.unwrap().thread().name());
            // Block until the worker yeilds a Sender and then push it into the sender queue
            let sender = rx.wait().next().unwrap();
            conn_queue.push_back(sender.unwrap());
        }

        QueryServer {
            conn_count: conn_count,
            query_queue: query_queue,
            conn_queue: Arc::new(Mutex::new(conn_queue))
        }
    }

    // queues up a query to execute that doesn't return a result
    pub fn execute(&mut self, query: String) {
        // no connections available
        if Arc::get_mut(&mut self.conn_queue).unwrap().get_mut().unwrap().is_empty() {
            // push query to the query queue
            println!("Locking query_queue in execute()");
            self.query_queue.lock().unwrap().push_back(query);
            println!("Query queued!");
        }else{
            let sender = Arc::get_mut(&mut self.conn_queue).unwrap().get_mut().unwrap().pop_front().unwrap();
            let cq_clone = self.conn_queue.clone();
            sender.send(Ok(query)).and_then(move |new_sender| {
                println!("Locking conn_queue in execute()");
                cq_clone.lock().unwrap().push_back(new_sender);
                Ok(())
            }).forget();
        }
    }
}
