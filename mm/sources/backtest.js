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

  // construct a backtest definition and send a StartBacktest command
  $("#backtestStartButton").click(function(){
    var symbol = $("#backtestSymbol").val();
    var start_time = $("#backtestStartTime").val();
    var end_time = $("#backtestEndTime").val();
    var type = $("#backtestTypeSelector").val();
    // Type should be a JSON-stringied `BacktestType`
    if(type == "Fast"){
      type = {Fast: {delay_ms: parseInt($("#backtestSendInterval").val())}};
    }
    var dataSrc = $("#backtestDataSrc").val();
    if(dataSrc == "Redis"){
      dataSrc = {Redis: {host: $("#redisSrcHost").val(), channel: $("#redisSrcChannel").val()}};
    }
    var dataDst = $("#backtestDataDst").val();
    if(dataDst == "Redis"){
      dataDst = {Redis: {host: $("#redisDstHost").val(), channel: $("#redisDstChannel").val()}};
    }
    // TODO: Configurable SimBroker settings
    var brokerSettings = {
      starting_balance: 50000.0,
      ping_ms: 0.2,
      execution_delay_us: 2,
    };

    // (start_timestamp, max_timestamp, max_tick_n, symbol, backtest_type, data_source, data_dest, broker_settings)
    var def = createBacktestDefinition(start_time, end_time, null, symbol, type, dataSrc, dataDst, brokerSettings);
    sendCommand("StartBacktest", "control", JSON.stringify({definition: def}), v4(), function(msg){
      if(msg.res == "Ok"){
        listBacktests();
        $("#commandRes").html("Backtest has been successfully started!");
      }
    });
  });

  // Show extended redis-only options only if Redis is selected.
  $("#backtestDataSrc").change(function(){
    if($("#backtestDataSrc").val() == "Redis"){
      $("#redisSrcOptions").show();
    } else {
      $("#redisSrcOptions").hide();
    }
  });

  $("#backtestDataDst").change(function(){
    if($("#backtestDataDst").val() == "Redis"){
      $("#redisDstOptions").show();
    } else {
      $("#redisDstOptions").hide();
    }
  });
});

/// Queries the Backtester module, requesting a list of running backtests
/// Updates the #activeBacktests tbody with the list.
function listBacktests(){
  sendCommand("Census", "control", "", v4(), function(msg){
    if(msg.res.Info){
      var list_ = JSON.parse(msg.res.Info.info);
      for(var i=0; i<list_.length; i++){
        if(list_[i].instance_type == "Backtester"){
          // this callback gets evalulated for every response received and
          // the backtest list gets written when a response from the Backtester is received
          sendCommand("ListBacktests", "control", "", v4(), function(msg2){
            if(msg2.res.Info){
              writeBacktests(JSON.parse(msg2.res.Info.info));
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

function writeBacktests(backtest_list){
  var html = "<tr><td>Backtest ID</td><td>Symbol</td></tr>";
  for(var i=0; i<backtest_list.length; i++){
    html += `<tr><td>${backtest_list[i].uuid}</td><td>${backtest_list[i].symbol}</td></tr>`;
  }
  if(backtest_list.length === 0){
    html += "<tr><td>No active backtests!</td></tr>";
  }
  $("#activeBacktests").html(html);
}

function setResponse(html) {
  $("#commandRes").html(html);
}

/// Creates a JSON-encoded String containing a backtest definition that can be send to the
/// backtester instance using the StartBacktest command.
///
/// Pass in null for things that should be None
function createBacktestDefinition(start_timestamp, max_timestamp, max_tick_n, symbol, backtest_type, data_source, data_dest, broker_settings) {
  // TODO: Configurable backtest start time propegated through the whole platform.
  if(start_timestamp !== null){
    start_timestamp = parseInt(start_timestamp);
  }
  if(max_timestamp !== null){
    max_timestamp = parseInt(max_timestamp);
  }

  var obj = {
    max_timestamp: max_timestamp,
    max_tick_n: max_tick_n,
    symbol: symbol,
    backtest_type: backtest_type,
    data_source: data_source,
    data_dest: data_dest,
    broker_settings: broker_settings,
  };

  return JSON.stringify(obj);
}
