//! Reads ticks out of a Postgres database

use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

use futures::sync::mpsc::{unbounded, UnboundedReceiver};
use postgres::Connection;
use postgres::rows::Rows;
use postgres::error::Error;
use algobot_util::trading::tick::*;
use algobot_util::transport::postgres::*;

use data::*;
use backtest::{BacktestCommand, BacktestMap};
use conf::CONF;

pub struct PostgresReader {
    pub symbol: String,
    pub start_time: Option<usize>,
}

impl TickGenerator for PostgresReader {
    fn get(
        &mut self, mut map: Box<BacktestMap + Send>, cmd_handle: CommandStream
    )-> Result<UnboundedReceiver<Tick>, String> {
        // small atomic communication bus between the handle listener and worker threads
        let internal_message: Arc<Mutex<BacktestCommand>> = Arc::new(Mutex::new(BacktestCommand::Stop));
        let got_mail = Arc::new(AtomicBool::new(false));
        let (mut tx, rx) = unbounded::<Tick>();

        let _got_mail = got_mail.clone();
        let symbol = self.symbol.clone();
        let start_time = self.start_time.clone();
        let reader_handle = thread::spawn(move || {
            let pg_conf = PostgresConf {
                postgres_user: CONF.postgres_user,
                postgres_password: CONF.postgres_password,
                postgres_url: CONF.postgres_url,
                postgres_port: CONF.postgres_port,
                postgres_db: CONF.postgres_db,
            };
            let conn_opt = get_client(pg_conf);
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
                        timestamp: row.get::<_, i64>(0) as usize,
                        bid: row.get::<_, i64>(1) as usize,
                        ask: row.get::<_, i64>(2) as usize,
                    };

                    // apply the map
                    let t_mod = map.map(tick);
                    if t_mod.is_some() {
                        tx.send(tick).unwrap();
                    }

                    // this should end up being the highest seen timestamp after the inner loop
                    cur_time = tick.timestamp;
                }
            }

            Ok(()) // ???
        }).thread().clone();

        // spawn the handle listener thread that awaits commands
        spawn_listener_thread(_got_mail, cmd_handle, internal_message, reader_handle);

        Ok(rx)
    }
}

pub fn get_ticks<'a>(symbol: &str, start_time: usize, conn: &'a Connection) -> Result<Rows<'a>, Error> {
    let query = format!(
        "SELECT (tick_time, bid, ask) FROM hist_{} WHERE tick_time > {} LIMIT 500 ORDER BY tick_time;",
        symbol,
        start_time
    );
    conn.query(&query, &[])
}
