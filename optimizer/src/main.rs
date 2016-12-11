//! Algobot 4 Optimizer
//! Created by Casey Primozic 2016-2016

#![feature(conservative_impl_trait, custom_derive, plugin, test)]

extern crate test;
extern crate uuid;
extern crate postgres;
extern crate redis;
extern crate futures;
extern crate serde;
extern crate serde_json;
extern crate fxcm;

extern crate algobot_util;
extern crate sample as strat;

mod conf;

use std::collections::HashMap;

use uuid::Uuid;
use algobot_util::transport::command_server::{CommandServer, CsSettings};
use algobot_util::transport::postgres::PostgresConf;
use algobot_util::transport::query_server::QueryServer;
use algobot_util::transport::commands::Command;
use algobot_util::strategies::Strategy;
use fxcm::FXCMNative;
use conf::CONF;

fn main() {
    let settings = CsSettings {
        redis_host: CONF.redis_host,
        responses_channel: CONF.redis_response_channel,
        conn_count: CONF.conn_senders,
        timeout: CONF.cs_timeout,
        max_retries: CONF.cs_max_retries
    };
    let mut cs = CommandServer::new(settings);
    let uuid = Uuid::new_v4();

    cs.execute(Command::Ready{
        instance_type: "Optimizer".to_string(),
        uuid: uuid,
    }, CONF.redis_commands_channel.to_string());

    let pg_conf = PostgresConf {
        postgres_user: CONF.postgres_user,
        postgres_password: CONF.postgres_password,
        postgres_url: CONF.postgres_url,
        postgres_port: CONF.postgres_port,
        postgres_db: CONF.postgres_db
    };
    let query_server = QueryServer::new(CONF.conn_senders, pg_conf);

    // initialize the strategy
    let mut broker = FXCMNative::new(HashMap::new());
    let mut strat = strat::new(cs, query_server, &mut broker);
    strat.init();
}
