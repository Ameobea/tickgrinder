//! Algobot 4 Optimizer
//! Created by Casey Primozic 2016-2016

#![allow(unconditional_recursion)]
#![feature(conservative_impl_trait, custom_derive, plugin, test)]
#![plugin(serde_macros)]

extern crate test;
extern crate uuid;
extern crate postgres;
extern crate redis;
extern crate futures;
extern crate serde;
extern crate serde_json;

extern crate algobot_util;
extern crate channel_id_sliding_windows;

mod conf;

use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::transport::postgres::PostgresConf;
use algobot_util::transport::query_server::QueryServer;
use algobot_util::strategies::Strategy;

// Set this line to the strategy to be used
use channel_id_sliding_windows::SlidingWindows as ActiveStrategy;

use conf::CONF;

fn main() {
    let settings = CsSettings {
        redis_host: CONF.redis_host,
        responses_channel: CONF.redis_response_channel,
        conn_count: CONF.conn_senders,
        timeout: CONF.cs_timeout,
        max_retries: CONF.cs_max_retries
    };
    let command_server = CommandServer::new(settings);

    let pg_conf = PostgresConf {
        postgres_user: CONF.postgres_user,
        postgres_password: CONF.postgres_password,
        postgres_url: CONF.postgres_url,
        postgres_port: CONF.postgres_port,
        postgres_db: CONF.postgres_db
    };
    let query_server = QueryServer::new(CONF.conn_senders, pg_conf);

    // initialize the strategy
    let mut strat = ActiveStrategy::new(command_server, query_server);
}
