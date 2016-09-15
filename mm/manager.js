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

manager.start = function(port){
  var app = express();

  var index = require('./routes/index');
  var api = require("./routes/api");

  app.engine('html', require('ejs').renderFile);
  app.set('views', path.join(__dirname, 'views'));
  app.set('view engine', 'ejs');
  app.use(bodyParser.json());
  app.use(bodyParser.urlencoded({extended: true}));
  app.listen(port);
  console.log("Manager webserver started!");

  app.use('/', index);
  app.use("/api", api);

  var socketServer = ws.createServer(function(conn){
    socketServer.on('error', function(err){
      console.log(`Websocket server had some sort of error: ${err}`);
    });

    conn.on('text', function(input){ //broadcast to all
      socketServer.connections.forEach(function(connection){
        connection.sendText(input);
      });
    });

    conn.on('close',function(code,reason){
      console.log('Websocket connection closed');
    });
  }).listen(parseInt(conf.websocketPort));

  // forwards messages from redis to the websocket
  var redisClient = redis.createClient();
  redisClient.subscribe("tickParserOutput");
  redisClient.on("message", (channel, message)=>{
    socketServer.connections.forEach(conn=>{
      conn.sendText(JSON.stringify({channel: channel, data: message}));
    });
  });

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

manager.start(conf.mmPort);
