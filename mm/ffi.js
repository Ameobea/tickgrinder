"use strict";
/*jslint node: true */

var ffi = require("ffi");
var ref = require("ref");
var StructType = require("ref-struct");
var ArrayType = require("ref-array");

var uint64_t = ref.types.uint64_t;
var UInt64_tArray = ArrayType(ref.types.uint64_t);

var Tick = StructType({
  timestamp: uint64_t,
  bid: uint64_t,
  ask: uint64_t,
});

var libalgobot = ffi.Library('libalgobot_util', {
  'ceil': [ 'double', [ 'double' ] ]
});

var Ffi = {

};

module.exports = Ffi;
