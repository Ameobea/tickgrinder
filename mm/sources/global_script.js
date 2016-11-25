//! Global script for all MM views.

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
