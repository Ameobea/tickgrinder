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
type SenderQueue = Arc<Mutex<VecDeque<Sender<(String, Sender<(), ()>), ()>>>>;
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
    client.execute(query.as_str(), &[])
        .map_err(|err| println!("Error saving tick: {:?}", err) );
    thread::sleep(Duration::new(6, 0));
}

// Creates a query processor that awaits requests
fn init_query_processor(rx: Receiver<(String, Sender<(), ()>), ()>, query_queue: QueryQueue) {
    // get a connection to the postgres database
    let client = get_client().expect("Couldn't create postgres connection.");
    // Handler for new queries from main thread
    // This blocks the worker thread until a new message is received
    // .wait() consumes the stream immediately, so the main thread has to wait
    // for the worker to push a message saying it's done before sending more messages
    for tup in rx.wait() {
        let (query, done_tx) = tup.unwrap();
        execute_query(query, &client);
        while let Some(new_query) = try_get_new_query(query_queue.clone()) {
            execute_query(new_query, &client);
        }
        // Let the main thread know it's safe to use the 
        done_tx.send(Ok(()));
    }
}

impl QueryServer {
    pub fn new(conn_count: usize) -> QueryServer {
        let mut conn_queue = VecDeque::with_capacity(conn_count);
        let query_queue = Arc::new(Mutex::new(VecDeque::new()));
        for i in 0..conn_count {
            // channel for getting the Sender back from the worker thread
            let (tx, rx) = channel::<(String, Sender<(), ()>), ()>();
            let qq_copy = query_queue.clone();
            thread::Builder::new().name(format!("worker{}", i))
                .spawn(move || { init_query_processor(rx, qq_copy) })
                .expect("Unable to Query Processor thread");
            // store the sender which can be used to send queries
            // to the worker in the connection queue
            conn_queue.push_back(tx);
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
        let temp_lock_res = self.conn_queue.lock().unwrap().is_empty();
        // Force the guard locking conn_queue to go out of scope
        // this prevents the lock from being held through the entire if/else
        let copy_res = temp_lock_res.clone();
        if copy_res {
            // push query to the query queue
            println!("Locking query_queue in execute()");
            self.query_queue.lock().unwrap().push_back(query);
            println!("Query queued!");
        }else{
            let tx = self.conn_queue.lock().unwrap().pop_front().unwrap();
            let cq_clone = self.conn_queue.clone();
            // channel for notifying main thread when query is done and worker is idle
            let (tx_tx, tx_rx) = channel::<(), ()>();
            tx.send(Ok((query, tx_tx))).and_then(|new_tx| {
                tx_rx.take(1).into_future().and_then(move |_| {
                    println!("Locking conn_queue in execute()");
                    cq_clone.lock().unwrap().push_back(new_tx);
                    println!("Pushing new Sender into conn_queue");
                    Ok(())
                }).forget();
                Ok(())
            }).forget();
        }
    }
}
