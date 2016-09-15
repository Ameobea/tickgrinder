"use strict";

var express = require("express");
var router = express.Router();
var http = require("http");
var redis = require("redis");

var conf = require("../conf");
// var backtest = require("../../backtest/backtest");
// var dbUtils = require("../../db_utils/utils");
// var spawner = require("../../utils/instance_spawner");

//URL: [ip]/api/backtest/[fast/live]/[pair]
//POST params: pair, startTime, [interval]
router.post("/backtest/start/:type/:pair", (req, res, next)=>{
  var type = req.params.type.toLowerCase();

  if(type == "fast"){
    res.send(backtest.fast(req.params.pair, req.body.startTime, req.body.interval));
  }else if(type == "live"){
    res.send(backtest.live(req.params.pair, req.body.startTime));
  }else if(type == "precalced"){
    res.send(backtest.precalced(req.params.pair, req.body.startTime, req.body.endTime));
  }else if(type == "nostore"){
    res.send(backtest.nostore(req.params.pair, req.body.startTime));
  }
});

//URL: [ip]/api/backtest/stop/[pair]
//POST params: NONE
router.post("/backtest/stop/:pair", (req, res, next)=>{
  if(req.params.pair != "all"){
    res.send(backtest.stop(req.params.pair.toLowerCase()));
  }else{
    res.send(backtest.clearFlags(()=>{}));
  }
});

//URL: [ip]/api/data/[pair]/[type]/[range]
//POST params: props = JSON-stringified object for mongo query
router.post("/data/:pair/:type/:range", (req, res, next)=>{
  dbUtils.fetchData(req.params.pair, req.params.type, JSON.parse(req.body.props), req.params.range, function(data){
    res.send(JSON.stringify(data));
  });
});

router.get("/utils/dbFlush/", (req, res, next)=>{
  dbUtils.flush(()=>{
    res.send("Database flushed.");
  });
});

router.get("/utils/dbDump", (req, res, next)=>{
  dbUtils.dump(()=>{
    res.send("Database dumped to file!");
  });
});

router.get("/utils/dbRestore/:dbRestoreId", (req, res, next)=>{
  dbUtils.load(req.params.dbRestoreId.trim(), (a,b,c)=>{
    res.send("Database dump loaded.");
  });
});

router.get("/instances", (req, res, next)=>{
  spawner.getInstances().then(data=>{
    res.send(JSON.stringify(data));
  }, (err)=>{console.log(err);});
});

router.get("/instances/kill/:type/:data", (req, res, next)=>{
  if(req.params.type == "manager"){
    http.get({hostname: conf.private.managerIp, port: parseInt(req.params.data), path: "/api/kill"}, resp=>{
      resp.on("data", data=>{
        res.send(data);
      });
    }).on('error', (e) => {
      res.send("That instance doesn't exist.");
    });
  }else if(req.params.type == "tickParser"){
    var redisPubClient = redis.createClient();
    var redisSubClient = redis.createClient();

    redisSubClient.subscribe("instanceCommands")

    redisSubClient.on("subscribe", ()=>{
      redisPubClient.publish("instanceCommands", JSON.stringify({command: "kill", id: req.params.data}));
    });

    redisSubClient.on("message", (channel, message)=>{
      var parsed = JSON.parse(message);

      if(parsed.status == "dying" && parsed.id == req.params.data){
        res.send(`Successfully killed instance with id ${req.params.data}`);
      }
    });
  }else if(req.params.type == "backtest"){
    dbUtils.mongoConnect(db=>{
      db.collection("backtestFlags").deleteOne({pair: req.params.data}).then(()=>{
        res.send(`Stopping backtest with pair ${req.params.data}...`);
      });
    });
  }
});

//:data should be a string in the following format: "EURUSD,USDCAD,USDJPY"
router.get("/instances/spawn/:type/:data", (req, res, next)=>{
  if(req.params.type == "tickParser"){
    spawner.spawnTickParser(req.params.data).then(result=>{
      res.send(JSON.stringify(result));
    });
  }
});

router.get("/conf/set/:name/:value/", (req, res, next)=>{
  try{
    conf.public[req.params.name] = eval(`${req.params.value}`);
    res.send("Config successfully updated.");
  }catch(e){
    res.send("Illegal entry recieved.  Make sure that you follow the guidelines in the usage section.");
  }
});

router.get("/conf/get", (req, res, next)=>{
  var confString = "<table>";

  for(var key in conf.public){
    if(!conf.public.hasOwnProperty(key)) continue;

    confString += `<tr><td><b>${key}</b>: ${conf.public[key]}</td>`;
    confString += `<td><input type="text" id="confInput-${key}"><input type="button" class="confSubmit" id="confSubmit-${key}" value="Update"></td>`;
    confString += `<td>${conf.desc.pub[key]}</td>`
  }

  confString += "</table>";

  res.send(confString);
})

router.get("/ping", (req, res, next)=>{
  res.send("pong");
});

router.get("/kill", (req, res, next)=>{
  res.send(JSON.stringify({success: true}));
  console.log("Message to kill manager recieved.  Suiciding.");
  setTimeout(()=>{
    process.exit(0);
  }, 1000);
});

module.exports = router;
