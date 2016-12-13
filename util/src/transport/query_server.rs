// Connection Pool-esque construct used to queue up
// and asynchronously execute Postgres queries

use std::collections::VecDeque;
use std::thread;
use std::sync::{Arc, Mutex};

use postgres;
use futures::{Future, Stream};
use futures::sync::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use futures::sync::oneshot::{channel as oneshot, Sender};

use transport::postgres::{get_client, PostgresConf};

type SenderQueue = Arc<Mutex<VecDeque<UnboundedSender<(String, Sender<()>)>>>>;
type QueryQueue = Arc<Mutex<VecDeque<String>>>;

pub struct QueryServer {
    query_queue: QueryQueue, // internal query queue
    conn_queue: SenderQueue, // senders for idle query threads
}

// locks the QueryQueue and returns a queued query, if there are any.
fn try_get_new_query(query_queue: &Mutex<VecDeque<String>>) -> Option<String> {
    let mut qq_inner = query_queue.lock().unwrap();
    qq_inner.pop_front()
}

// executes the query and blocks the calling thread until it completes
fn execute_query(query: &str, client: &postgres::Connection) {
    let _ = client.execute(query, &[]);
}

// Creates a query processor that awaits requests
fn init_query_processor(rx: UnboundedReceiver<(String, Sender<()>)>, query_queue: QueryQueue,
        pg_conf: PostgresConf) {
    // get a connection to the postgres database
    let client = get_client(pg_conf).expect("Couldn't create postgres connection.");
    // Handler for new queries from main thread
    // This blocks the worker thread until a new message is received
    // .wait() consumes the stream immediately, so the main thread has to wait
    // for the worker to push a message saying it's done before sending more messages
    for tup in rx.wait() {
        let (query, done_tx) = tup.unwrap();
        execute_query(query.as_str(), &client);
        // keep trying to get queued queries to execute until the queue is empty
        while let Some(new_query) = try_get_new_query(&*query_queue) {
            execute_query(new_query.as_str(), &client);
        }
        // Let the main thread know it's safe to use the sender again
        // This essentially indicates that the worker thread is idle
        done_tx.complete(());
    }
}

impl QueryServer {
    pub fn new(conn_count: usize, pg_conf: PostgresConf) -> QueryServer {
        let mut conn_queue = VecDeque::with_capacity(conn_count);
        let query_queue = Arc::new(Mutex::new(VecDeque::new()));
        for _ in 0..conn_count {
            let _pg_conf = pg_conf.clone();
            // channel for getting the Sender back from the worker thread
            let (tx, rx) = unbounded::<(String, Sender<()>)>();
            let qq_copy = query_queue.clone();
            thread::spawn(move || init_query_processor(rx, qq_copy, _pg_conf) );
            // store the sender which can be used to send queries
            // to the worker in the connection queue
            conn_queue.push_back(tx);
        }

        QueryServer {
            query_queue: query_queue,
            conn_queue: Arc::new(Mutex::new(conn_queue))
        }
    }

    // Queues up a query to execute that doesn't return a result.
    pub fn execute(&mut self, query: String) {
        // no connections available
        let temp_lock_res = self.conn_queue.lock().unwrap().is_empty();
        // Force the guard locking conn_queue to go out of scope
        // this prevents the lock from being held through the entire if/else
        let copy_res = temp_lock_res.clone();
        if copy_res {
            // push query to the query queue
            self.query_queue.lock().unwrap().push_back(query);
        } else {
            let mut tx = self.conn_queue.lock().unwrap().pop_front().unwrap();
            let cq_clone = self.conn_queue.clone();
            // future for notifying main thread when query is done and worker is idle
            let (c, o) = oneshot::<()>();
            thread::spawn(move || {
                tx.send( (query, c) ).unwrap();
                // Wait until the worker thread signals that it is idle
                let _ = o.wait();
                // Put the Sender for the newly idle worker into the connection queue
                cq_clone.lock().unwrap().push_back(tx);
            });
        }
    }
}
