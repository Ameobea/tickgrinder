//! Tick Writer
//!
//! Listens for new, live, real ticks on redis and records them to flatfile storage for backtesting etc.

/*jslint node: true */
"use strict";

var redis = require("redis");
var fs = require("fs");

//var conf = require("../../conf/conf");

var redisClient = redis.createClient();
redisClient.subscribe("ticks");

var initFile = (symbol, callback)=>{
  fs.writeFile("./" + symbol + ".csv", "timestamp, bid, ask", (err, res)=>{
    callback();
  });
};

var existingFiles = {};

var parsed;
redisClient.on("message", (channel, message)=>{
  parsed = JSON.parse(message);
  console.log(message);

  if(parsed.real){
    new Promise((fulfill, reject)=>{
      if(!existingFiles[parsed.symbol]){
        fs.stat("./" + parsed.symbol + ".csv", (err, stat)=>{
          if(err){
            initFile(parsed.symbol, ()=>{
              existingFiles[parsed.symbol] = true;
              fulfill();
            });
          }else{
            existingFiles[parsed.symbol] = true;
            fulfill();
          }
        });
      }else{
        fulfill();
      }
    }).then(()=>{
      var appendString = "\n" + parsed.timestamp.toString() + ", " + parsed.bid.toString() + ", " + parsed.ask.toString();
      fs.appendFile("./" + parsed.symbol + ".csv", appendString, (err, res)=>{});
    });
  }
});

