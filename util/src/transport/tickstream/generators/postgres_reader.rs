//! Reads ticks out of a Postgres database

use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

use futures::{Future, Stream, Sink};
use futures::stream::BoxStream;
use futures::sync::mpsc::channel;
use postgres::Connection;
use postgres::rows::Rows;
use postgres::error::Error;

use trading::tick::*;
use transport::postgres::*;

use super::super::*;

pub struct PostgresReader {
    pub symbol: String,
    pub start_time: Option<u64>,
}

impl TickGenerator for PostgresReader {
    fn get(
        &mut self, mut map: Box<TickMap + Send>, cmd_handle: CommandStream
    ) -> Result<BoxStream<Tick, ()>, String> {
        // small atomic communication bus between the handle listener and worker threads
        let internal_message: Arc<Mutex<TickstreamCommand>> = Arc::new(Mutex::new(TickstreamCommand::Stop));
        let got_mail = Arc::new(AtomicBool::new(false));
        let (mut tx, rx) = channel::<Tick>(1);

        let _got_mail = got_mail.clone();
        let symbol = self.symbol.clone();
        let start_time = self.start_time;
        let reader_handle = thread::spawn(move || {
            let conn_opt = get_client();
            if conn_opt.is_err() {
                return Err("Unable to create Postgres client".to_string())
            }
            let conn = conn_opt.unwrap();

            let mut cur_time = start_time.or(Some(0)).unwrap();
            loop {
                let rows_opt = get_ticks(&symbol, cur_time, &conn);
                if rows_opt.is_err() {
                    println!("Got an error back from Postgres when trying to get ticks");
                    break;
                }
                let rows = rows_opt.unwrap();
                for row in rows.iter() {
                    let tick = Tick {
                        timestamp: row.get::<_, i64>(0) as u64,
                        bid: row.get::<_, i64>(1) as usize,
                        ask: row.get::<_, i64>(2) as usize,
                    };

                    // apply the map
                    let t_mod = map.map(tick);
                    if t_mod.is_some() {
                        tx = tx.send(tick).wait().expect("Unable to send through tx in `get` in postgres_reader!");
                    }

                    // this should end up being the highest seen timestamp after the inner loop
                    cur_time = tick.timestamp;
                }
            }

            Ok(()) // ???
        }).thread().clone();

        // spawn the handle listener thread that awaits commands
        spawn_listener_thread(_got_mail, cmd_handle, internal_message, reader_handle);

        Ok(rx.boxed())
    }

    fn get_raw(&mut self) -> Result<BoxStream<Tick, ()>, String> {
        let (mut tx, rx) = channel(1);

        let conn_opt = get_client();
        if conn_opt.is_err() {
            return Err("Unable to create Postgres client".to_string())
        }
        let conn = conn_opt.unwrap();

        let mut cur_time = self.start_time.or(Some(0)).unwrap();
        let symbol = self.symbol.clone();
        thread::spawn(move ||{
            loop {
                let rows_opt = get_ticks(&symbol, cur_time, &conn);
                if rows_opt.is_err() {
                    println!("Got an error back from Postgres when trying to get ticks");
                    break;
                }
                let rows = rows_opt.unwrap();
                for row in rows.iter() {
                    let tick = Tick {
                        timestamp: row.get::<_, i64>(0) as u64,
                        bid: row.get::<_, i64>(1) as usize,
                        ask: row.get::<_, i64>(2) as usize,
                    };

                    tx = tx.send(tick).wait().expect("Unable to send through tx in `get_raw` in potgres_reader!");

                    // this should end up being the highest seen timestamp after the inner loop
                    cur_time = tick.timestamp;
                }
            }
        });

        Ok(rx.boxed())
    }
}

fn get_ticks<'a>(symbol: &str, start_time: u64, conn: &'a Connection) -> Result<Rows<'a>, Error> {
    let query = format!(
        "SELECT (tick_time, bid, ask) FROM hist_{} WHERE tick_time > {} LIMIT 500 ORDER BY tick_time;",
        symbol,
        start_time
    );
    conn.query(&query, &[])
}
