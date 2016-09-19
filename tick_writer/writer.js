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

var initFile = (pair, callback)=>{
  fs.writeFile("./" + pair + ".csv", "timestamp, bid, ask", (err, res)=>{
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
      if(!existingFiles[parsed.pair]){
        fs.stat("./" + parsed.pair + ".csv", (err, stat)=>{
          if(err){
            initFile(parsed.pair, ()=>{
              existingFiles[parsed.pair] = true;
              fulfill();
            });
          }else{
            existingFiles[parsed.pair] = true;
            fulfill();
          }
        });
      }else{
        fulfill();
      }
    }).then(()=>{
      var appendString = "\n" + parsed.timestamp.toString() + ", " + parsed.bid.toString() + ", " + parsed.ask.toString();
      fs.appendFile("./" + parsed.pair + ".csv", appendString, (err, res)=>{});
    });
  }
});
