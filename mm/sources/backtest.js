//! Scripts for the backtests view of the MM

/*jslint browser: true*/ /*global $, initWs, registerCheck, sendCommand, v4 */
"use strict";

var socket;

$(document).ready(function(){
  socket = initWs(function(wr_msg){
    var msg = wr_msg.message;
    // check for registered interest
    if(msg.uuid && msg.res){
      registerCheck(msg.uuid, msg);
    }
    if(msg.cmd && msg.cmd.Ready){
      listBacktests();
    }
  });

  socket.onopen = function(event){
    listBacktests();
  };
});

// TODO: Button to initialize a backtester instance

/// Queries the Backtester module, requesting a list of running backtests
/// Updates the #activeBacktests tbody with the list.
function listBacktests(){
  sendCommand("Census", "control", "", v4(), function(msg){
    if(msg.res.Info){
      var list_ = JSON.parse(msg.res.Info.info);
      for(var i=0; i<list_.length; i++){
        if(list_[i].instance_type == "Backtester"){
          $("#activeBacktests").html("<tr><td>TODO</td></tr>");
          sendCommand("ListBacktests", "control", "", v4(), function(msg2){
            if(msg2.res.Info){
              // TODO
            }
          });
          return;
        }
      }
      var message = "<tr><td>No backtester running!  Click the button to the right to start an instance.</td>";
      message += `<td><button onclick=\"sendCommand(\'SpawnBacktester\', \'control\', \'\', \'${v4()}\', function(){})\">`;
      message += "Start Backtester</button></td></tr>";
      $("#activeBacktests").html(message);
    }
  });
}

function setResponse(text) {}
