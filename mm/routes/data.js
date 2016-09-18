//! Library of functions for loading data for use in plotting charts.
//! Data is loaded from whatever sources necessary (primarily database) and
//! returned in a format that can be loaded directly into a chart.

/*jslint node: true */
"use strict";

var express = require("express");
var router = express.Router();
var pg = require("pg");
var conf = require("../conf");

var config = {
  user: conf.postgresUser,
  database: conf.postgresDatabase,
  password: conf.postgresPassword,
  host: conf.posgresUrl,
  port: conf.posrgresPort,
  max: 10,
  idleTimeoutMillis: 835672,
};
var pool = new pg.Pool(config);

/// Returns raw ticks with price being the average of the bid and ask.
router.get("/ticks/:symbol/:start/:end/:data", (req, res, next)=>{
  var query = `SELECT * FROM ticks_${req.params.symbol} WHERE tick_time > ${req.params.start} AND tick_time < ${req.params.end};`;
  console.log(query);

  // Use dummy data during development
  res.json({data: [[10000,2],[20000,2.31],[30000,3.12]],
    name: req.params.symbol + " Prices"});

  // pool.connect((err, client, done)=>{
  //   client.query(query, (err, _res)=>{
  //     if(!err){
  //       res.send(_res);
  //     } else {
  //       res.send(err);
  //     }
  //     done();
  //   });
  // });
});

/// Returns two sets of data, one for both the bid and ask.
router.get("/bidask/:symbol/:start/:end/:data", (req, res, next)=>{
  // Use dummy data during development
  res.json({lower: [[10000,2],[20000,2.31],[30000,3.12]],
    upper: [[10000,2.1],[20000,2.41],[30000,3.22]],
    lower_name: req.params.symbol + " Bids",
    upper_name: req.params.symbol + " Asks"});
});

/// Returns SMA with the given period (if it exists in the database)
router.get("/sma/:symbol/:start/:end/:data", (req, res, next)=>{
  // Use dummy data during development
  res.json({data: [[10000,2],[20000,2.31],[30000,3.12]]});
});

module.exports = router;
