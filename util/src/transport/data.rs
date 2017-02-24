//! Utilities related to the transfer of data from one place to another.  Handles the conversion of data from one format
//! to another and from one storage type to another.

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::prelude::*;
use std::io::BufReader;
use std::fmt;
use std::thread;

use serde_json;
use redis;
use libc::{uint64_t, c_double};
use postgres::Connection;

use transport::commands::HistTickDst;
use transport::redis::get_client as get_redis_client;
use transport::postgres::get_client as get_postgres_client;
use transport::postgres::init_hist_data_table;
use transport::query_server::QueryServer;
use transport::command_server::CommandServer;
use trading::tick::Tick;
use conf::CONF;

/// Initializes the transfer of data from a `HistTickGen` to a `HistTickDst`.  Data is read into an internal buffer within
/// the generator and then written into the sink.
pub fn transfer_data(src: HistTickDst, dst: HistTickDst, cs: CommandServer) {
    thread::spawn(move || {
        let tx_iterator = get_tx_iterator(src, cs);
        let mut rx_closure = get_rx_closure(dst).unwrap();

        for tick in tx_iterator {
            rx_closure(tick);
        }
    });
}

/// Given a `HistTickDst`, returns a closure that can be used as a receiver callback.
pub fn get_rx_closure(dst: HistTickDst) -> Result<RxCallback, String> {
    let cb = match dst.clone() {
        HistTickDst::Console => {
            let inner = |t: Tick| {
                println!("{:?}", t);
            };

            RxCallback{
                dst: dst,
                inner: Box::new(inner),
            }
        },
        HistTickDst::RedisChannel{host, channel} => {
            let client = get_redis_client(host.as_str());
            // buffer up 5000 ticks in memory and send all at once to avoid issues
            // with persistant redis connections taking up lots of ports
            let mut buffer: Vec<String> = Vec::with_capacity(5000);

            let inner = move |t: Tick| {
                let client = &client;
                let tick_string = serde_json::to_string(&t).unwrap();
                buffer.push(tick_string);

                // Send all buffered ticks once the buffer is full
                if buffer.len() >= 5000 {
                    let mut pipe = redis::pipe();
                    for item in buffer.drain(..) {
                        pipe.cmd("PUBLISH")
                            .arg(&channel)
                            .arg(item);
                    }
                    pipe.execute(client);
                }
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
        HistTickDst::RedisSet{host, set_name} => {
            let client = get_redis_client(host.as_str());
            // buffer up 5000 ticks in memory and send all at once to avoid issues
            // with persistant redis connections taking up lots of ports
            let mut buffer: Vec<String> = Vec::with_capacity(5000);

            let inner = move |t: Tick| {
                let client = &client;
                let tick_string = serde_json::to_string(&t).unwrap();
                buffer.push(tick_string);

                // Send all buffered ticks once the buffer is full
                if buffer.len() >= 5000 {
                    let mut pipe = redis::pipe();
                    for item in buffer.drain(..) {
                        pipe.cmd("SADD")
                            .arg(&set_name)
                            .arg(item);
                    }
                    pipe.execute(client);
                }
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
        HistTickDst::Flatfile{filename} => {
            let fnc = filename.clone();
            let path = Path::new(&fnc);
            // create the file if it doesn't exist
            if !path.exists() {
                let _ = File::create(path).unwrap();
            }

            // try to open the specified filename in append mode
            let file_opt = OpenOptions::new().append(true).open(path);
            if file_opt.is_err() {
                return Err(format!("Unable to open file with path {}", filename));
            }
            let mut file = file_opt.unwrap();

            let inner = move |t: Tick| {
                let tick_string = t.to_csv_row();
                file.write_all(tick_string.as_str().as_bytes())
                    .expect(format!("couldn't write to output file: {}, {}", filename, tick_string).as_str());
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
        HistTickDst::Postgres{table} => {
            let connection_opt = get_postgres_client();
            if connection_opt.is_err() {
                return Err(String::from("Unable to connect to PostgreSQL!"))
            }
            let connection = connection_opt.unwrap();
            try!(init_hist_data_table(table.as_str(), &connection, CONF.postgres_user));
            let mut qs = QueryServer::new(10);

            let mut inner_buffer = Vec::with_capacity(5000);

            let inner = move |t: Tick| {
                let val = format!("({}, {}, {})", t.timestamp, t.bid, t.ask);
                inner_buffer.push(val);
                if inner_buffer.len() > 4999 {
                    let mut query = String::from(format!("INSERT INTO {} (tick_time, bid, ask) VALUES ", table));
                    let values = inner_buffer.as_slice().join(", ");
                    query += &values;
                    query += ";";

                    qs.execute(query);
                    inner_buffer.clear();
                }
            };

            RxCallback {
                dst: dst,
                inner: Box::new(inner),
            }
        },
    };

    Ok(cb)
}

/// A struct that functions as a callback for ticks in a generator.
pub struct RxCallback {
    dst: HistTickDst,
    inner: Box<FnMut(Tick)>,
}

impl FnOnce<(Tick,)> for RxCallback {
    type Output = ();
    extern "rust-call" fn call_once(self, args: (Tick,)) {
        let mut inner = self.inner;
        inner(args.0)
    }
}

impl FnMut<(Tick,)> for RxCallback {
    extern "rust-call" fn call_mut(&mut self, args: (Tick,)) {
        (*self.inner)(args.0)
    }
}

impl fmt::Debug for RxCallback {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RxCallback: {:?}",  self.dst)
    }
}

pub struct TxCallback {
    inner: Box<FnMut(uint64_t, c_double, c_double)>,
}

impl FnOnce<(uint64_t, c_double, c_double,)> for TxCallback {
    type Output = ();
    extern "rust-call" fn call_once(self, args: (uint64_t, c_double, c_double,)) {
        let mut inner = self.inner;
        inner(args.0, args.1, args.2)
    }
}

impl FnMut<(uint64_t, c_double, c_double,)> for TxCallback {
    extern "rust-call" fn call_mut(&mut self, args: (uint64_t, c_double, c_double,)) {
        (*self.inner)(args.0, args.1, args.2)
    }
}

fn get_tx_iterator(src: HistTickDst, cs: CommandServer) -> Box<HistTickGen> {
    match src {
        HistTickDst::Flatfile{filename} => {
            Box::new(FlatfileReader::new(filename, cs))
        },
        HistTickDst::Postgres{table} => {
            Box::new(PostgresReader::new(table.to_string(), cs))
        }
        _ => unimplemented!(),
    }
}

/// This trait is implemented by objects that generate historical ticks from stored source.
pub trait HistTickGen {
    /// Signal the tick generator to populate its internal tick buffer.
    fn populate_buffer(&mut self) -> Result<(), String>;

    /// Gets a mutable reference to the generator's internal tick buffer.
    fn get_buffer(&mut self) -> &mut Vec<Tick>;

    /// Tick generators must provide access to a `CommandServer` for logging purposes
    fn get_cs(&mut self) -> &mut CommandServer;
}

impl Iterator for HistTickGen {
    type Item = Tick;

    fn next(&mut self) -> Option<Tick> {
        if self.get_buffer().is_empty() {
            match self.populate_buffer() {
                Ok(_) => (),
                Err(err) => self.get_cs().error(Some("Tick Loading"), &format!("Error while loading ticks into buffer: {}", err)),
            };
        }

        self.get_buffer().pop()
    }
}

/// A historical tick reader that draws upon a CSV file as a data source
struct FlatfileReader {
    buffer: Vec<Tick>,
    buf_reader: BufReader<File>,
    cs: CommandServer,
}

impl HistTickGen for FlatfileReader {
    fn get_buffer(&mut self) -> &mut Vec<Tick> {
        &mut self.buffer
    }

    /// Reads lines out of the file to fill the buffer
    fn populate_buffer(&mut self) -> Result<(), String> {
        assert_eq!(self.buffer.len(), 0);

        for _ in 0..500 {
            let mut buf = String::new();
            let _ = self.buf_reader.read_line(&mut buf).unwrap();
            let tick = Tick::from_csv_string(&buf);
            self.buffer.push(tick);
        }

        Ok(())
    }

    fn get_cs(&mut self) -> &mut CommandServer {
        &mut self.cs
    }
}

impl FlatfileReader {
    pub fn new(filename: String, cs: CommandServer) -> FlatfileReader {
        let path = Path::new(&filename);
        let file = File::open(path).expect(&format!("Unable to open file at {:?}", path));
        let mut reader = BufReader::new(file);
        // skip header row
        let _ = reader.read_line(&mut String::new());
        FlatfileReader {
            buf_reader: reader,
            buffer: Vec::with_capacity(500),
            cs: cs,
        }
    }
}

/// A historical tick generator that draws upon a PostgreSQL table as its data source
struct PostgresReader {
    buffer: Vec<Tick>,
    conn: Connection,
    last_timestamp: usize,
    table_name: String,
    cs: CommandServer,
}

impl HistTickGen for PostgresReader {
    fn get_buffer(&mut self) -> &mut Vec<Tick> {
        &mut self.buffer
    }

    /// Queries the database and populates the buffer with rows from the database
    fn populate_buffer(&mut self) -> Result<(), String> {
        assert_eq!(self.buffer.len(), 0);
        let query = format!(
            "SELECT (tick_time, bid, ask) FROM {} WHERE tick_time > {} LIMIT 500;",
            self.table_name,
            self.last_timestamp
        );
        let rows = self.conn.query(&query, &[]).map_err(|x| format!("{:?}", x))?;

        for (i, row) in rows.iter().enumerate() {
            let t = Tick {
                timestamp: row.get::<usize, i64>(0) as u64,
                bid: row.get::<usize, i64>(1) as usize,
                ask: row.get::<usize, i64>(2) as usize,
            };
            self.buffer[i] = t;
        }

        Ok(())
    }

    fn get_cs(&mut self) -> &mut CommandServer {
        &mut self.cs
    }
}

impl PostgresReader {
    pub fn new(table_name: String, cs: CommandServer) -> PostgresReader {
        let conn = get_postgres_client();

        PostgresReader {
            buffer: Vec::with_capacity(500),
            conn: conn.unwrap(),
            last_timestamp: 0,
            table_name: table_name,
            cs: cs,
        }
    }
}
