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

  // TODO: Update to dynamically determined ip
  var serverAddress = "http://<%= ip %>";

  var socket = new WebSocket("ws://<%= websocketIp %>");
  socket.onmessage = message=>{
    processWsMsg(JSON.parse(message.data));
  };

  $("#backtestStartButton").click(()=>{
    var type = $("#backtestTypeSelector option:selected").text().toLowerCase();

    if(type == "fast"){
      let requestAddress = serverAddress + "api/backtest/start/fast/" + $("#backtestStartPair").val();
      $.post(requestAddress, {startTime: $("#backtestStartTime").val(), interval: $("#backtestSendInterval").val()});
    }else if(type == "live"){
      let requestAddress = serverAddress + "api/backtest/start/live/" + $("#backtestStartPair").val();
      $.post(requestAddress, {startTime: $("#backtestStartTime").val()});
    }else if(type == "pre-calculated"){
      let requestAddress = serverAddress + "api/backtest/start/precalced/" + $("#backtestStartPair").val();
      $.post(requestAddress, {startTime: $("#backtestStartTime").val(), endTime: $("#backtestEndTime").val()});
    }else if(type == "no-store"){
      let requestAddress = serverAddress + "api/backtest/start/nostore/" + $("#backtestStartPair").val();
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
    var requestAddress = serverAddress + "api/backtest/stop/" + $("#backtestStopPair").val();
    $.post(requestAddress);
  });

  $("#backtestStopAllButton").click(()=>{
    var requestAddress = serverAddress + "api/backtest/stop/all";
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

//Message format:
//{channel: "sample", data:"{id: "stuff", data:"stringified JSON"}"}
var processWsMsg = msg=>{
  if(msg.channel == "tickParserOutput"){
    //console.log(msg.data);
    var data = JSON.parse(msg.data);
    var id = `#tickParser-${data.id}`;
    if(typeof($(id).html()) != "undefined"){
      console.log($(id).html());
      $(id).append(data.data);
    }else{
      if(dispQ[id]){
        dispQ[id].push(data.data);
      }else{
        dispQ[id] = [data.data];
      }
    }
  }
};

function decodeEntities(input){
  var y = document.createElement('textarea');
  y.innerHTML = input;
  return y.value;
}

var loadConfig = ()=>{
  <%
    var confString = "<table>";

    for(var key in conf.public){
      if(!conf.public.hasOwnProperty(key)) continue;

      confString += `<tr><td><b>${key}</b>: ${conf.public[key]}</td>`;
      confString += `<td><input type="text" id="confInput-${key}">`;
      confString += `<input type="button" class="confSubmit" id="confSubmit-${key}" value="Update"></td>`;
    }

    confString += "</table>";
  %>

  $("#config").html(decodeEntities(`<%= confString %>`));

  setupConfListeners();
};

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

var update = ()=>{
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

var getKillButton = (type, data)=>{
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

var killConfirm = (type, data)=>{
  var res = window.confirm("Are you sure you want to kill this instance?");
  if(res){
    killInstance(type, data);
  }
};

var killInstance = (type, data)=>{
  $.get(`./api/instances/kill/${type}/${data}`, res=>{
    $("#statusbar").html(res);
    update();
  }).fail(err=>{
    $("#statusbar").html(`<p>Failed to kill instance: ${err.status}</p>`);
  });
};

var spawnInstance = (type, data)=>{
  $.get(`./api/instances/spawn/${type}/${data}`, res=>{
    res = JSON.parse(res);
    $("#statusbar").html(`Instance with id ${res.id} spawned with message: ${res.data}`);
    setTimeout(()=>{update();},500);
  });
};
