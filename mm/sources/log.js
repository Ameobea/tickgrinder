"use strict";
/*jslint browser: true*/ /*global $, console, initWs, registerCheck, sendCommand, v4 */

// globals
var socket;
var virt_table = [];

$(document).ready(function(){
  // set up websocket callbacks
  socket = initWs(processWsMsg);
});

/// Parses the messages into JSON, displays them in the messages list, and handles registered interest.
function processWsMsg(wr_msg) {
  var msg = wr_msg.message;
  if(msg.cmd && msg.cmd.Log){ // is a log message
    msg.cmd.Log.msg.timestamp = Date.now();
    virt_table.unshift(msg.cmd.Log.msg);
    if(virt_table.length > 30){
      virt_table.pop();
    }

    writeTable();
  }
}

/// Writes all the logged messages into the table
function writeTable() {
  var res = "";
  for(var i=0;i<virt_table.length;i++){
    let msg = virt_table[i];
    res += `<tr><td>${msg.timestamp}</td><td>${msg.sender.instance_type}</td><td>${msg.level}</td><td>${msg.message_type}</td><td>${msg.message}</td></tr>`;
  }
  $("#log-body").html(res);
}
