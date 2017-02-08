//! Functions for interfacing with the platform's Redis-based communication system

const CONF = require("../conf.js");

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

/// Starts the WS listening for new messages sets up processing callback
function initWs(callback, dispatch) {
  var regex = /https?:\/\/([^\/:]*)/g;
  var socketUrl = "ws://" + regex.exec(document.URL)[1] + ":7037";
  var socket = new WebSocket(socketUrl);
  socket.onmessage = message=>{
    callback(dispatch, JSON.parse(message.data));
  };
  return socket;
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

export default {
  initWs: initWs,
  getResponse: getResponse,
  v4: v4,
}
