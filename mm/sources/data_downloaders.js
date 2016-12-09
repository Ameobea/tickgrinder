//! Scripts for the Data Downloaders page
/*jslint browser: true*/ /*global $, initWs, registerCheck, sendCommand, v4, console */
"use strict";

var socket;

$(document).ready(function(){
  socket = initWs(function(wr_msg){
    var msg = wr_msg.message;
    // check for registered interest
    if(msg.uuid && msg.res){
      registerCheck(msg.uuid, msg);
    } else if (msg.cmd && msg.cmd.Ready){
      writeDataDownloaders(function(){});
    } else if (msg.cmd && (msg.cmd.DownloadTicks || msg.cmd.DownloadComplete)){
      setTimeout(function(){
        writeDataDownloaders(function(){});
      }, 1192);
    }

    $("#data-dst").change(function(){
      var val = $("#data-dst").val();
      $("#flatfile-config").hide();
      $("#redis-channel-config").hide();
      $("#redis-set-config").hide();
      $("#postgres-config").hide();

      if(val == "flatfile"){
        $("#flatfile-config").show();
      } else if(val == "redis-channel") {
        $("#redis-channel-config").show();
      } else if(val == "redis-set") {
        $("#redis-set-config").show();
      } else if(val == "postgres") {
        $("#postgres-config").show();
      }
    });
  });

  socket.onopen = function(event){
    writeBoth();
  };
});

/// Populates the active downloaders table with data from the platform.  Calls back
/// with false if there are no running data downloaders and a list of the [DownloaderType, UUID]s
/// of all downloader instances otherwise.
function writeDataDownloaders(cb) {
  getDownloaderUuids(function(ids){
    if(ids.length === 0){
      $("#active_instances").html("<tr><td>No active data downloaders!</td></tr>");
      cb(false);
      return;
    }

    var html = "<tr><td>Instance Type</td><td>UUID</td></tr>";
    for(var i=0; i<ids.length; i++){
      html += `<tr><td>${ids[i][0]}</td><td>${ids[i][1]}</td></tr>`;
      $("#download-list").append(`<option value="${ids[i][1]}">${ids[i][1]}</option>`);
    }
    $("#active_instances").html(html);
    cb(ids);
  });
}

/// Returns a list of the [DownloaderType, UUID]s of all downloader instances and calls back with it.
function getDownloaderUuids(callback) {
  sendCommand("Census", "control", "", v4(), function(msg){
    if(msg.res.Info){
      var list_ = JSON.parse(msg.res.Info.info);
      var uuid_list = [];
      for(var i=0; i<list_.length; i++){
        // if "Data Downloader" is in the instance type
        if(list_[i].instance_type.indexOf("Data Downloader") != -1){
          uuid_list.push([list_[i].instance_type, list_[i].uuid]);
        }
      }
      callback(uuid_list);
    }
  });
}

function writeBoth() {
  writeDataDownloaders(function(ids){
    if(ids){
      $("#active_downloads").html("<tr><td>Downloader Type</td><td>Symbol</td><td>Start Time</td><td>End Time</td><td>Data Destination</td></tr>");
      // for each of the active data downloaders
      for(var i=0; i<ids.length; i++){
        // can't use i anymore because JS is async and i will have changed to 1 by the time the callback is triggered
        // I found that out the hard way.
        var ii = i;
        sendCommand("ListRunningDownloads", ids[ii][1], "", v4(), function(res){
          if(res.res && res.res.Info){
            var list_ = res.res.Info.info;
            var downloads = JSON.parse(list_);
            // for each of the running downloads on that downloader
            for(var j=0; j<downloads.length; j++){
              var item = downloads[j];
              $("#active_downloads").append(
                `<tr><td>${ids[ii][0]}</td><td>${item.symbol}</td><td>${item.start_time}</td><td>${item.end_time}</td><td>${JSON.stringify(item.dst)}</td></tr>`
              );
            }
          }
        });
      }
    } else {
      $("#active_downloads").html("<tr><td>No active downloads! </td></tr>");
    }
  });
}

function startDownload() {
  var val = $("#data-dst").val();
  var channel = $("#download-list").val();
  var args = {
    symbol: $("#symbol").val(),
    start_time: $("#start-time").val(),
    end_time: $("#end-time").val(),
    dst: {},
  };

  if(val == "flatfile"){
    args.dst.Flatfile = {filename: $("#flatfile-filename").val()};
  } else if(val == "redis-channel") {
    args.dst.RedisChannel = {
      host: $("#redis-channel-host").val(),
      channel: $("#redis-channel").val(),
    };
  } else if(val == "redis-set") {
    args.dst.RedisSet = {
      host: $("#redis-set-host").val(),
      set_name: $("#redis-set").val(),
    };
  } else if(val == "postgres") {
    args.dst.Postgres = {table: $("#postgres-table").val()};
  }

  sendCommand("DownloadTicks", channel, JSON.stringify(args), v4(), function(res){
    if(res.res == "Ok"){
      writeBoth();
    }
  });
}

function spawnDataDownloader(type) {
  if(type == "FXCM"){
    sendCommand("SpawnFxcmDataDownloader", "control", "", v4(), function(){});
  }
}

function setResponse(html) {
  console.log(html);
}
