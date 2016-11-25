//! Scripts for the backtests view of the MM

/*jslint browser: true*/ /*global $, initWs, registerCheck, sendCommand, v4 */
"use strict";

$(document).ready(function(){
  initWs(function(msg){
    // check for registered interest
    if(msg.uuid && msg.res){
      registerCheck(msg.uuid, msg);
    }
  });
});

// TODO: Button to initialize a backtester instance

/// Queries the Backtester module, requesting a list of running backtests
/// Updates the #activeBacktests tbody with the list.
function listBacktests(){
  sendCommand("ListBacktests", "control", "", v4(), function(msg){
    if(msg.res.Info){
      // TODO
    }
  });
}
