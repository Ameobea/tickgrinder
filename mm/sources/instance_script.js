"use strict";
/*jslint browser: true*/ /*global $*/

var dispQ = {};

$(document).ready(()=>{
  //set up button listeners
  $("#instance-spawn-button").click(()=>{
    var type = $("#instance-type").val();
    var data = $("#instance-data-box").val();

    spawnInstance(type, data);
  });

  var socketUrl = (document.URL.replace(/https*:\/\//g, "ws://") + ":7037").replace(/:\d+\/*/, "");
  var socket = new WebSocket(socketUrl);
  socket.onmessage = message=>{
    processWsMsg(JSON.parse(message.data));
  };

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

  update();
  reloadConfig();
});

// Message format:
// {channel: "sample", data:{uuid: uuid, cmd/res: {...}}}
function processWsMsg(msg) {
  console.log(msg);
};

function decodeEntities(input){
  var y = document.createElement('textarea');
  y.innerHTML = input;
  return y.value;
}

var setupConfListeners = ()=>{
  $(".confSubmit").click(function(){
    var id = this.attributes.getNamedItem("id").value.split("-")[1];
    var val = $(`#confInput-${id}`).val();

    $.get(`./api/conf/set/${id}/${val}`, data=>{
      $("#configStatus").html(`<p>${data}</p>`);
      reloadConfig();
    });
  });
};

var reloadConfig = ()=>{
  $.get("./api/conf/get", data=>{
    $("#config").html(decodeEntities(data));

    setupConfListeners();
  });
};

function update(){
  $.get("./api/instances", data=>{
    var parsed = JSON.parse(data);
    var table = "<table><tr><th>Kill</th><th>Type</th><th>Data</th></tr>";

    $.each(parsed.instances, (index, elem)=>{
      if(elem.type == "manager"){
        table += `<tr><td>${getKillButton(elem.type, elem)}</td>`;
        table += `<td>Manager</td><td>Port: <b>${elem.port}</b></td></tr>`;
      }else if(elem.type == "tickParser"){
        var id = `#tickParser-${elem.id}`;
        var msg = "";
        var first = true;

        if(dispQ[id]){
          dispQ[id].forEach(line=>{
            msg += `${first ? "" : "<br>"}${line}`;
            first = false;
          });
          dispQ[id] = false;
        }

        table += `<tr><td>${getKillButton(elem.type, elem)}</td>`;
        table += `<td>Tick Parser</td><td>Listening for pairs: <b>${elem.pairs}</b></td>`;
        table += `<td id="${id}">${msg ? msg : ""}</td></tr>`;
      }
    });

    $.each(parsed.backtests, (index, elem)=>{
      table += `<tr><td>${getKillButton("backtest", elem)}</td>`;
      table += `<td>Backtest</td><td>Pair: <b>${elem.pair}</b></td></tr>`;// TODO: Add type to backtest
    });

    table += "</table>";

    $("#running-instances").html(table);
  }).fail(err=>{
    console.log("Unable to connect to API");
  });
};

function getKillButton(type, data) {
  var include;

  if(type == "manager"){
    include = data.port;
  }else if(type == "tickParser"){
    include = data.id;
  }else if(type == "backtest"){
    include = data.pair;
  }

  return `<input type="button" onclick="killConfirm('${type}', '${include}')" value="X">`;
};

function killConfirm(type, data) {
  var res = window.confirm("Are you sure you want to kill this instance?");
  if(res){
    killInstance(type, data);
  }
};

function killInstance(type, data) {
  $.get(`./api/instances/kill/${type}/${data}`, function(res) {
    $("#statusbar").html(res);
    update();
  }).fail(err=>{
    $("#statusbar").html(`<p>Failed to kill instance: ${err.status}</p>`);
  });
};

function spawnInstance(type, data) {
  $.get(`./api/instances/spawn/${type}/${data}`, function(res) {
    res = JSON.parse(res);
    $("#statusbar").html(`Instance with id ${res.id} spawned with message: ${res.data}`);
    setTimeout(()=>{update();},500);
  });
};
