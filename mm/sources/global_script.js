//! Global script for all MM views.

"use strict";

var interest = [];

/// Starts the WS listening for new messages sets up processing callback
function initWs(callback) {
  var regex = /https?:\/\/([^\/:]*)/g;
  var socketUrl = "ws://" + regex.exec(document.URL)[1] + ":7037";
  var socket = new WebSocket(socketUrl);
  socket.onmessage = message=>{
    callback(JSON.parse(message.data));
  };
  return socket;
}

/// Checks if a UUID is registered and calls the callback if it is
function registerCheck(uuid, msg){
  for(var i=0;i<interest.length;i++){
    if(interest[i][0] == uuid){
      interest[i][1](msg);
    }
  }
}

/// Deregisters a UUID from the interest array
function deregister(uuid){
  for(var i=0;i<interest.length;i++){
    if(interest[i][0] == uuid){
      interest = interest.splice(i, 1);
      return;
    }
  }
}

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

/// Sends a command to the platform.  Params is a JSON-formatted String.
function sendCommand(command, channel, params, uuid, callback) {
  setResponse("");
  if(params.length != 0 && params != "{}") {
    try {
      params = JSON.parse(params);
    } catch(e) {
      setResponse("Unable to parse params into valid JSON object: " + params);
      return;
    }
    var command_ = {};
    command_[command] = params;
    command = command_;
  }

  // register the callback
  interest.push([uuid, callback]);

  var msgObj = {channel: channel, message: {uuid: uuid, cmd: command}};
  socket.send(JSON.stringify(msgObj));

  // remove the registered callback after the timeout expires
  setTimeout(function(){
    deregister(uuid);
  }, resTimeoutMs);
}
