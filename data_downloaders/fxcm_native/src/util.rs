//! Various utilities for manipulating historical ticks.

use std::thread;
use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};

use postgres::Connection;

use super::get_rx_closure;

use tickgrinder_util::trading::tick::*;
use tickgrinder_util::transport::postgres::*;
use tickgrinder_util::transport::commands::HistTickDst;

pub fn transfer_data(src: HistTickDst, dst: HistTickDst) {
    thread::spawn(move || {
        let tx_iterator = get_tx_iterator(src);
        let mut rx_closure = get_rx_closure(dst).unwrap();

        for tick in tx_iterator {
            rx_closure(tick);
        }
    });
}

fn get_tx_iterator(src: HistTickDst) -> Box<HistTickGen> {
    match src {
        HistTickDst::Flatfile{filename} => {
            Box::new(FlatfileReader::new(filename))
        },
        HistTickDst::Postgres{table} => {
            Box::new(PostgresReader::new(table.to_string()))
        }
        _ => unimplemented!(),
    }
}

pub trait HistTickGen {
    fn populate_buffer(&mut self);

    fn get_buffer(&mut self) -> &mut Vec<Tick>;
}

impl Iterator for HistTickGen {
    type Item = Tick;

    fn next(&mut self) -> Option<Tick> {
        if self.get_buffer().is_empty() {
            self.populate_buffer();
        }
        self.get_buffer().pop()
    }
}

struct FlatfileReader {
    buffer: Vec<Tick>,
    buf_reader: BufReader<File>,
}

impl HistTickGen for FlatfileReader {
    fn get_buffer(&mut self) -> &mut Vec<Tick> {
        &mut self.buffer
    }

    /// Reads lines out of the file to fill the buffer
    fn populate_buffer(&mut self) {
        assert_eq!(self.buffer.len(), 0);

        for _ in 0..500 {
            let mut buf = String::new();
            let _ = self.buf_reader.read_line(&mut buf).unwrap();
            let tick = Tick::from_csv_string(&buf);
            self.buffer.push(tick);
        }
    }
}

impl FlatfileReader {
    pub fn new(filename: String) -> FlatfileReader {
        let path = Path::new(&filename);
        let file = File::open(path).expect(&format!("Unable to open file at {:?}", path));
        let mut reader = BufReader::new(file);
        // skip header row
        let _ = reader.read_line(&mut String::new());
        FlatfileReader {
            buf_reader: reader,
            buffer: Vec::with_capacity(500),
        }
    }
}

struct PostgresReader {
    buffer: Vec<Tick>,
    conn: Connection,
    last_timestamp: usize,
    table_name: String,
}

impl HistTickGen for PostgresReader {
    fn get_buffer(&mut self) -> &mut Vec<Tick> {
        &mut self.buffer
    }

    /// Queries the database and populates the buffer with rows from the database
    fn populate_buffer(&mut self) {
        assert_eq!(self.buffer.len(), 0);
        let query = format!(
            "SELECT (tick_time, bid, ask) FROM {} WHERE tick_time > {} LIMIT 500;",
            self.table_name,
            self.last_timestamp
        );
        let rows = self.conn.query(&query, &[])
            .expect("Error in query");

        for (i, row) in rows.iter().enumerate() {
            let t = Tick {
                timestamp: row.get::<usize, i64>(0) as u64,
                bid: row.get::<usize, i64>(1) as usize,
                ask: row.get::<usize, i64>(2) as usize,
            };
            self.buffer[i] = t;
        }
    }
}

impl PostgresReader {
    pub fn new(table_name: String) -> PostgresReader {
        let conn = get_client();

        PostgresReader {
            buffer: Vec::with_capacity(500),
            conn: conn.unwrap(),
            last_timestamp: 0,
            table_name: table_name,
        }
    }
}
