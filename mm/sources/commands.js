//! Populates the command interface with live commands + responses, and handles
//! command creation and sending.

"use strict";
/*jslint node: true */
/*jslint browser: true*/ /*global Highcharts, $*/

$(document).ready(function(){
  $("#send").off().click(function(){
    sendCommand();
  });
  initWs();
});

function initWs() {
  var socketUrl = (document.URL.replace(/https*:\/\//g, "ws://") + ":7037").replace(/:\d+\/*\w*/, "");
  var socket = new WebSocket(socketUrl);
  socket.onmessage = message=>{
    processWsMsg(JSON.parse(message.data));
  };
}

function processWsMsg(parsedMsg) {
  // TODO
}

/// Reads the values from the form and sends the command
function sendCommand() {
  setResponse("");
  var command = $("#command").val();
  var channel = $("#channel").val();
  var bcast   = $("#broadcast").is(":checked");
  var params  = $("#params").val();
  if(params === "") {
    params = "{}";
  }
  try {
    params = JSON.parse(params);
  } catch(e) {
    setResponse("Unable to parse params into valid JSON object");
  }
  // TODO
}

function setResponse(text) {
  $("#response").html(text);
}
