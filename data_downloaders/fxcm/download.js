//! FXCM Historical Data Downloader
//!
//! Using the FXCM Broker API, this utility pulls down historical ticks from their trade servers
//! in conjunction with the tick_recorder java application which serves as the link to their API.
//!
//! 1. Queue up many requests and save their unique ids to RequestQueue
//! 2. java connector returns a new chunkID when the API accepts the request, save that to DownloadQueue
//! 3. new segment returned with segment's data; add success to SuccessQueue
//! 4. after it's been 1 second since last response from the server, re-queue up any un-recieved requests.
//! 5. repeat step 4 until all chunks in the superSegment are recieved; then move on to next superSegment.

/*jslint node: true */
"use strict";

var redis = require("redis"); //TODO: Intelligently skip weekends
var fs = require("fs");
var uuid64 = require("uuid64");

var Promise = require("bluebird");
Promise.onPossiblyUnhandledRejection(function(error){
    throw error;
});

//var conf = require("../../conf/conf");

//TODO: Enable resuming from last tick in output file
//TODO: Make start/stop time cli arguments or create config file

//unix timestamp format.
var symbol = "usdcad"; //like "usdcad"
var startTime = 1467766209497; //like 1393826400 * 1000
var endTime = 1473757200 * 1000;

//time between data requests
var downloadDelay = 50;

//ms to wait after last data from server to re-send missed requests
var checkDelay = 300;

//0 = no logging, 1 = error logging, 2 = log EVERYTHING
var logLevel = 0;

//number of 10-second chunks queued up (almost) simultaneously
var superChunkSize = 50;

var redisPubclient = redis.createClient();
var redisSubClient = redis.createClient();
redisSubClient.subscribe("historicalPrices");

var requestQueue; // holds uuids of sent segment requests
var downloadQueue; // holds ids and uuids of accepted segment requests
var successQueue; // holds ids of downloaded segments

var lastDataMs; // time in ms of last recieved data from server

redisSubClient.on("message", (channel, message)=>{
  if(logLevel == 2){
    console.log("getting: " + message);
  }
  var parsed = JSON.parse(message);

  if(parsed.error == "No ticks in range"){
    lastDataMs = Date.now();
    successQueue.push(parsed.id);
  }else if(parsed.status && parsed.status == ">300 data"){ //there were more than 300 ticks in the 10-second range
    //TODO: Handle >300 ticks
    if(logLevel >= 1){
      console.log("Error - more than 300 ticks in that 10-second time range.");
    }
  }else if(parsed && parsed.type == "segmentID"){ // segment request recieved and download started
    downloadQueue.push({uuid: parsed.uuid, id: parsed.id});
  }else if(parsed && parsed.type == "segment"){// new segment
    lastDataMs = Date.now();
    successQueue.push(parsed.id);

    parsed.data.forEach(tick=>{
      storeTick(tick);
    });
  }
});

var formatSymbol = rawSymbol=>{
  var currencyOne = rawSymbol.toUpperCase().substring(0,3);
  var currencyTwo = rawSymbol.toUpperCase().substring(3,6);
  return currencyOne + "/" +  currencyTwo;
};

//Starts the download of a 10-second segment of ticks.
var downloadSegment = (startTime, delay)=>{
  setTimeout(()=>{
    var uuid = uuid64();
    //console.log(uuid);

    var toSend = {uuid: uuid, symbol: formatSymbol(symbol), startTime: startTime, endTime: startTime + 10000, resolution: "t1"};
    requestQueue.push(toSend);

    redisPubclient.publish("priceRequests", JSON.stringify([toSend]));
  });
};


//This queues up a ton of 10-second chunks to download semi-asynchronously.
var downloadSuperSegment = startTime=>{
  resetQueues();

  for(var i=0;i<superChunkSize;i++){
    downloadSegment(startTime, i*downloadDelay);

    startTime = startTime + 10000;
  }

  lastDataMs = false;
  downloadWaiter().then(()=>{
    verifyDownload().then(()=>{
      downloadSuperSegment(startTime);
    });
  });
};

var verifyDownload = ()=>{
  return new Promise((f,r)=>{
    var verify = ()=>{
      let toResend = [];

      requestQueue.forEach(request=>{
        //Thanks to https://github.com/dalexj for these sexy lines:
        let filtered = downloadQueue.filter(download => download.uuid === request.uuid);
        let downloadMatches = filtered.length >= 1;

        // If the request was received by the server
        if(downloadMatches){
          // If no response was received
          if(!successQueue.includes(filtered[0].id)){
            console.log("resending " + request.uuid);
            toResend.push(request);
          }
        }else{ // server never got our request
          toResend.push(request);
        }
      });

      if(toResend.length > 0){
        resetQueues();
        lastDataMs = false;

        toResend.forEach(elem=>{
          requestQueue.push(elem);
          redisPubclient.publish("priceRequests", JSON.stringify([elem]));
        });

        downloadWaiter().then(verify);
      }else{
        f();
      }
    };

    verify();
  });
};

var downloadWaiter = ()=>{
  return new Promise((f,r)=>{
    var check = ()=>{
      if(typeof lastDataMs == "number" && Date.now() >= lastDataMs + checkDelay){
        f();
      }else{
        setTimeout(check, checkDelay/10);
      }
    };

    check();
  });
};

var resetQueues = ()=>{
  requestQueue = [];
  downloadQueue = [];
  successQueue = [];
};

var existingFiles = {};
var toAppend;

var storeTick = (tick)=>{
  if(tick.timestamp > endTime){
    console.log("All ticks in range downloaded and stored.");
    process.exit(0);
  }

  new Promise((fulfill, reject)=>{
    if(!existingFiles[symbol]){
      fs.stat("./" + symbol + ".csv", (err, res)=>{
        if(err){
          initFile(symbol, ()=>{
            existingFiles[symbol] = true;
            fulfill();
          });
        }else{
          existingFiles[symbol] = true;
          fulfill();
        }
      });
    }else{
      fulfill();
    }
  }).then(()=>{
    toAppend = "\n" + tick.timestamp + ", " + tick.bid + ", " + tick.ask;
    fs.appendFile("./" + symbol + ".csv", toAppend, (err, res)=>{});
  });
};

var initFile = (symbol, callback)=>{
  fs.writeFile("./" + symbol + ".csv", "timestamp, bid, ask", (err, res)=>{
    callback();
  });
};

downloadSuperSegment(startTime); //initiate segment downloading

