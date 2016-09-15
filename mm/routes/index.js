"use strict";
var express = require("express");
var router = express.Router();
var conf = require("../../conf/conf");

/* GET home page. */
router.get('/', (req, res, next)=>{
  res.render("instances", {ip: conf.public.managerServerIP+":"+conf.public.managerServerPort+"/", conf: conf});
});

router.get('/sources/:file', (req, res, next)=>{
  res.render("sources/" + req.params.file.split(".")[0], {websocketIp: conf.private.websocketIp, ip: conf.public.managerServerIP+":"+conf.public.managerServerPort+"/", conf: conf});
});

router.get("/monitor", (req, res, next)=>{
  res.render("monitor", {conf: conf});
});

router.get("/instances", (req, res, next)=>{
  res.render("instances");
});

module.exports = router;
