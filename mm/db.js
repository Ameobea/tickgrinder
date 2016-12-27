"use strict";
/*jslint node: true */

var pg = require('pg').native;
var Promise = require("bluebird");

const CONF = require("./conf");

var pool_conf = {
  user: CONF.postgresUser,
  database: CONF.postgres_database,
  password: CONF.postgres_password,
  host: CONF.postgres_url,
  port: CONF.postgres_port,
  max: 10,
  idleTimeoutMillis: 30000,
};

var accessors = {
  get_tick_time_differences(pool, table, start_time, end_time){
    return new Promise((f,r)=>{
      pool.connect(function(err, client, done) {
        if(err) {
          r(console.error('error fetching client from pool', err));
          return;
        }
        var query =
            `SELECT tick_time - LAG(tick_time) OVER (ORDER BY tick_time) AS time_diff,
             bid - LAG(bid) OVER(ORDER BY tick_time) AS bid_diff,
             ask - LAG(ask) OVER(ORDER BY tick_time) AS ask_diff
             FROM ${table} WHERE tick_time > ${start_time} AND tick_time < ${end_time};`;
        client.query(query, [], (err, res)=>{
          done();

          if(err) {
            r(console.error('error running query', err));
          } else {
            f(res.rows);
          }
        });
      });
    });
  }
};

var pool = new pg.Pool(pool_conf).on('error', function (err, client) {
  console.error('idle client error', err.message, err.stack);
});

var Db = {
  pool: pool,
  accessors: accessors,
};

module.exports = Db;
