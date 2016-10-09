"use strict";
/*jslint browser: true*/ /*global $*/

var resTimeoutMs = 3000;
var squelchPings = true;
// how many messages are currently displayed in the messages box
var outputLen = 0;
var socket;
// array of [UUID, callback]s of responses we're interested in
var interest = [];
var dispQ = {};
var defaultCallback = function(msg){
  var oldResText = $("#response").html();
  setResponse(oldResText + "<br>" + JSON.stringify(msg.res));
};

$(document).ready(function(){
  // set up the listeners for the manual message sending
  $("#send").off().click(function(){
    var command = $("#command").val();
    var channel = $("#channel").val();
    var params  = $("#params").val();
    var uuid    = v4();

    sendCommand(command, channel, params, uuid, defaultCallback);
  });

  squelchPings = $("#squelchpings").is(":checked");
  $("#squelchpings").off().change(function(){
    squelchPings = !squelchPings;
  });

  // set up websocket callbacks
  initWs();

  //set up button listeners
  $("#instance-spawn-button").click(()=>{
    var type = $("#instance-type").val();
    var data = $("#instance-data-box").val();

    spawnInstance(type, data);
  });

  // set up backtest listeners
  $("#backtestStartButton").click(()=>{
    var type = $("#backtestTypeSelector option:selected").text().toLowerCase();

    if(type == "fast"){
      let requestAddress = "../api/backtest/start/fast/" + $("#backtestStartPair").val();
      $.post(requestAddress, {startTime: $("#backtestStartTime").val(), interval: $("#backtestSendInterval").val()});
    }else if(type == "live"){
      let requestAddress = "../api/backtest/start/live/" + $("#backtestStartPair").val();
      $.post(requestAddress, {startTime: $("#backtestStartTime").val()});
    }else if(type == "pre-calculated"){
      let requestAddress = "../api/backtest/start/precalced/" + $("#backtestStartPair").val();
      $.post(requestAddress, {startTime: $("#backtestStartTime").val(), endTime: $("#backtestEndTime").val()});
    }else if(type == "no-store"){
      let requestAddress = "../api/backtest/start/nostore/" + $("#backtestStartPair").val();
      $.post(requestAddress, {startTime: $("#backtestStartTime").val()});
    }
  });

  $("#backtestTypeSelector").change(()=>{
    if($("#backtestTypeSelector option:selected").text().toLowerCase() == "pre-calculated"){
      $("#endtime").show();
    }else{
      $("#endtime").hide();
    }
  });

  $("#backtestStopButton").click(()=>{
    var requestAddress = "../api/backtest/stop/" + $("#backtestStopPair").val();
    $.post(requestAddress);
  });

  $("#backtestStopAllButton").click(()=>{
    var requestAddress = "../api/backtest/stop/all";
    $.post(requestAddress);
  });

  $("#dbFlush").click(()=>{
    $.get("api/utils/dbFlush");
  });

  $("#dbDump").click(()=>{
    $.get("api/utils/dbDump");
  });

  $("#dbRestore").click(()=>{
    var restoreId = $("#dbRestoreId").val();
    $.get("api/utils/dbRestore/" + restoreId);
  });

  // populate the list of running instances
  setTimeout(function(){
    update();
  }, 1212)
});

/// Starts the WS listening for new messages sets up processing callback
function initWs() {
  var socketUrl = (document.URL.replace(/https*:\/\//g, "ws://") + ":7037")
    .replace(/:\d+\/*\w*/, "");
  socket = new WebSocket(socketUrl);
  socket.onmessage = message=>{
    processWsMsg(JSON.parse(message.data));
  };
}

/// Parses the messages into JSON, displays them in the messages list, and handles registered interest.
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

  // check for registered interest
  if(msg.uuid && msg.res){
    registerCheck(msg.uuid, msg);
  }
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

/// Reads the values from the form and sends the command
function sendCommand(command, channel, params, uuid, callback) {
  setResponse("");
  if(params === "") {
    params = "{}";
  }
  try {
    params = JSON.parse(params);
  } catch(e) {
    setResponse("Unable to parse params into valid JSON object");
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

/// Sets the HTML of the Responses box
function setResponse(text) {
  $("#response").html(text);
}

/// Sends a command to get the list of running instances and
/// sets up the callback for when the list is received
function update(){
  var command = "Census";
  var channel = "control";
  var params = "{}";
  var uuid = v4();
  // called after a response is received
  var callback = function(msg){
    // is a valid Census response
    if(msg.res.Info){
      writeInstances(JSON.parse(msg.res.Info.info));
    }
  };

  sendCommand(command, channel, params, uuid, callback);
}

/// Writes the list of instances to the page
function writeInstances(instances){
  var table = "<table><tr><th>Kill</th><th>Type</th><th>Data</th></tr>";

  console.log(instances);
  $.each(instances, (index, elem)=>{
    table += `<tr><td>${getKillButton(elem.uuid)}</td>`;
    table += `<td>${elem.instance_type}</td><td>TODO</td></tr>`;
  });

  table += "</table>";

  $("#running-instances").html(table);
}

/// Creates a button that, when clicked, kills the instance its affiliated with
function getKillButton(uuid) {
  return `<input type="button" onclick="killConfirm('${uuid}')" value="X">`;
}

/// Creates a prompt that asks the user if they really want to kill the instance
function killConfirm(uuid, data) {
  var res = window.confirm("Are you sure you want to kill this instance?");
  if(res){
    killInstance(uuid);
  }
}

/// Sends the command to kill the instance with the supplied UUID
function killInstance(uuid) {
  sendCommand("Kill", uuid, "{}", v4(), defaultCallback);
}

/// Spawns an instance of the specified type
function spawnInstance(type, data) {
  // TODO
}
