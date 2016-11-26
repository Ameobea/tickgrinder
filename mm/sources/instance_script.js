"use strict";
/*jslint browser: true*/ /*global $, initWs, registerCheck, sendCommand, v4 */

var squelchPings = true;
// how many messages are currently displayed in the messages box
var outputLen = 0;
var socket;
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
  socket = initWs(processWsMsg);

  //set up button listeners
  $("#instance-spawn-button").click(()=>{
    var type = $("#instance-type").val();
    var data = $("#instance-data-box").val();

    spawnInstance(type, data);
  });

  // populate the list of running instances
  setTimeout(function(){
    update();
  }, 1212);
});

/// Parses the messages into JSON, displays them in the messages list, and handles registered interest.
function processWsMsg(wr_msg) {
  var msg = wr_msg.message;
  if(!(squelchPings && ((msg.cmd && msg.cmd == "Ping") || (msg.res && msg.res.Pong)))) {
    var oldhtml = $("#cmdres").html();
    var split = oldhtml.split("\n");
    outputLen += 1;
    if(outputLen > 10) {
      split = split.splice(1); // remove first element (oldest message)
    }
    split.push(`<tr><td style="color:#0066ff">${wr_msg.channel}</td><td style="color:#009933">${JSON.stringify(msg)}</td></tr>`);
    $("#cmdres").html(split.join("\n"));
  }

  // check for registered interest
  if(msg.uuid && msg.res){
    registerCheck(msg.uuid, msg);
  }

  // a new instance has spawned so update instance list
  if(msg.cmd && msg.cmd.Ready){
    update();
  }
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
  var uuid = v4();
  // called after a response is received
  var callback = function(msg){
    // is a valid Census response
    if(msg.res.Info){
      writeInstances(JSON.parse(msg.res.Info.info));
    }
  };

  sendCommand(command, channel, "", uuid, callback);
}

/// Writes the list of instances to the page
function writeInstances(instances){
  var table = "<table><tr><th>Kill</th><th>Type</th><th>Data</th></tr>";

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
  sendCommand("Kill", uuid, "{}", v4(), function(msg){
    defaultCallback(msg);
    if(msg.res.Info){
      setTimeout(update, 4000);
    }
  });
}

/// Spawns an instance of the specified type
function spawnInstance(type, data) {
  switch(type){
    case "tick_parser":
      sendCommand("SpawnTickParser", "control", JSON.stringify({symbol: data}), v4(), function(msg){
        defaultCallback(msg);
      });
      break;
    case "backtester":
      sendCommand("SpawnBacktester", "control", "", v4(), function(msg){
        defaultCallback(msg);
      });
      break;
  }
}
