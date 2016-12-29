"use strict";
var express = require("express");
var router = express.Router();
var conf = require("../conf");

router.get("/", (req, res, next)=>{
  res.render("instances", { conf: conf });
});

router.get("/monitor", (req, res, next)=>{
  res.render("monitor", { conf: conf });
});

router.get("/instances", (req, res, next)=>{
  res.render("instances", { conf: conf });
});

router.get("/backtest", (req, res, next)=>{
  res.render("backtest", { conf: conf });
});

router.get("/data_downloaders", (req, res, next)=>{
  res.render("data_downloaders", { conf: conf });
});

router.get("/log", (req, res, next)=>{
  res.render("log", { conf: conf });
});

module.exports = router;
