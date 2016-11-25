"use strict";
var express = require("express");
var router = express.Router();
var conf = require("../conf");

router.get("/", (req, res, next)=>{
  res.render("instances");
});

router.get("/monitor", (req, res, next)=>{
  res.render("monitor");
});

router.get("/instances", (req, res, next)=>{
  res.render("instances");
});

router.get("/backtest", (req, res, next)=>{
  res.render("backtest");
});

module.exports = router;
