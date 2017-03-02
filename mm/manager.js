"use strict";
/*jslint node: true */

var express = require("express");
var path = require("path");
var bodyParser = require("body-parser");
var http = require("http");
var ws = require("nodejs-websocket");
var redis = require("redis");

var conf = require("./conf");

var manager = exports;

var uuid;

/// Generates a new V4 UUID in hyphenated form
function v4() {
  function s4() {
    return Math.floor((1 + Math.random()) * 0x10000)
      .toString(16)
      .substring(1);
  }
  return s4() + s4() + '-' + s4() + '-' + s4() + '-' +
    s4() + '-' + s4() + s4() + s4();
}

manager.start = function(port){
  var app = express();

  var index = require('./routes/index');
  var data = require("./routes/data");

  app.engine('html', require('ejs').renderFile);
  app.set('views', path.join(__dirname, 'views'));
  app.set('view engine', 'ejs');
  app.use(bodyParser.json());
  app.use(bodyParser.urlencoded({extended: true}));
  app.listen(port, "0.0.0.0");
  console.log("Manager webserver started!");

  app.use("/", index);
  app.use("/data", data);
  app.use("/sources", express.static(__dirname + "/sources"));

  var pubClient = getRedisClient();

  var socketServer = ws.createServer(function(conn){
    socketServer.on('error', function(err){
      console.log(`Websocket server had some sort of error: ${err}`);
    });

    conn.on('text', function(txtMsg){ //broadcast to all
      socketServer.connections.forEach(function(connection){
        connection.sendText(txtMsg);
      });

      try {
        var parsed = JSON.parse(txtMsg);
        if(parsed.channel && parsed.message && parsed.message.cmd){
          pubClient.publish(parsed.channel, JSON.stringify(parsed.message));
        }
      } catch(e) {}
    });
  }).listen(parseInt(conf.websocket_port), "0.0.0.0");

  // usage: node manager.js uuid
  uuid = process.argv[2];

  if(!uuid) {
    console.error("Usage: node manager.js uuid");
    process.exit(0);
  } else {
    console.error(`MM now listening for commands on ${conf.redis_control_channel} and ${uuid}`);
  }

  // Create two Redis clients - one for subscribing and one for publishing
  var subClient = getRedisClient();

  subClient.subscribe(uuid);
  subClient.subscribe(conf.redis_control_channel);
  subClient.subscribe(conf.redis_responses_channel);
  subClient.subscribe(conf.redis_log_channel);
  subClient.on("message", (channel, message_str)=>{
    // convert the {"Enum"}s to plain strings
    message_str = message_str.replace(/{("\w*")}/g, "$1");
    var wr_msg = JSON.parse(message_str);
    // broadcast to websockets
    socketServer.connections.forEach(function(connection){
      var ws_msg = {channel: channel, message: wr_msg};
      connection.sendText(JSON.stringify(ws_msg));
    });
    if(wr_msg.cmd && !wr_msg.cmd.Log){
      var response = getResponse(wr_msg.cmd);
      var wr_res = {uuid: wr_msg.uuid, res: response};
      pubClient.publish(conf.redis_responses_channel, JSON.stringify(wr_res));
    }
  });

  // signal to the platform that we're up and running
  setTimeout(function(){
    pubClient.publish(conf.redis_control_channel, JSON.stringify({uuid: v4(), cmd: {Ready: {instance_type: "MM", uuid: uuid}}}));
  }, conf.cs_timeout);

  app.use(function(err, req, res, next) {
    res.status(err.status || 500);
    console.log(err.stack);
    res.render('error', {
      message: err.message,
      error: err
    });
  });

  app.use(function(req, res, next) {
    res.status(404).send('Resource not found');
  });
};

manager.start(conf.mm_port);

/// Returns a new Redis client based on the settings in conf
function getRedisClient() {
  var spl = conf.redis_host.split("://")[1].split(":");
  return redis.createClient({
    host: spl[0],
    port: parseInt(spl[1]),
  });
}

/// Processes a command and returns a Response to send back
function getResponse(command) {
  switch(command) {
    case "Ping":
      var temp = JSON.parse(JSON.stringify(process.argv));
      return {Pong: {args: temp.splice(2)}};
    case "Kill":
      // shut down in 3 seconds
      setTimeout(function() {
        console.log("MM is very tired...");
        process.exit(0);
      }, 3000);
      return {Info: {info: "Shutting down in 3 seconds..."}};
    case "Type":
      return {Info: {info: "MM"}};
    default:
      return {Error: {status: "Command not recognized."}};
  }
}
