//! Populates the command interface with live commands + responses, and handles
//! command creation and sending.

"use strict";
/*jslint node: true */
/*jslint browser: true*/ /*global Highcharts, $*/

var squelchPings = true;
var outputLen = 0;

$(document).ready(function(){
  $("#send").off().click(function(){
    sendCommand();
  });

  squelchPings = $("#squelchpings").is(":checked");
  $("#squelchpings").off().change(function(){
    squelchPings = !squelchPings;
  });

  initWs();
});

function initWs() {
  var socketUrl = (document.URL.replace(/https*:\/\//g, "ws://") + ":7037")
    .replace(/:\d+\/*\w*/, "");
  var socket = new WebSocket(socketUrl);
  socket.onmessage = message=>{
    processWsMsg(JSON.parse(message.data));
  };
}

function processWsMsg(wr_msg) {
  console.log(wr_msg);
  var msg = wr_msg.message;
  if(!(squelchPings && ((msg.cmd && msg.cmd == "Ping") || (msg.res && msg.res.Pong)))) {
    var oldhtml = $("#cmdres").html();
    var split = oldhtml.split("<br>");
    outputLen += 1;
    if(outputLen > 10) {
      split = split.splice(1); // remove first element (oldest message)
    }
    split.push(`<b>Channel: </b>${wr_msg.channel}; <b>Message: </b> ${JSON.stringify(msg)}`);

    $("#cmdres").html(split.join("<br>"));
  }
  // TODO: handle registered listeners
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
