//! Functions for interfacing with the platform's Redis-based communication system

const CONF = require("../conf.js");

/// Generates a new V4 UUID in hyphenated form
function v4() {
  function s4() {
    return Math.floor((1 + Math.random()) * 0x10000)
      .toString(16)
      .substring(1);
  }
  return s4() + s4() + '-' + s4() + '-' + s4() + '-' +
    s4() + '-' + s4() + s4() + s4();
}

/// Returns a new Redis client based on the settings in conf
function getRedisClient() {
  var spl = conf.redis_host.split("://")[1].split(":");
  return redis.createClient({
    host: spl[0],
    port: parseInt(spl[1]),
  });
}

/// Processes a command and returns a Response to send back
function getResponse(command) {
  switch(command) {
    case "Ping":
      var temp = JSON.parse(JSON.stringify(process.argv));
      return {Pong: {args: temp.splice(2)}};
    case "Kill":
      // shut down in 3 seconds
      setTimeout(function() {
        console.log("MM is very tired...");
        process.exit(0);
      }, 3000);
      return {Info: {info: "Shutting down in 3 seconds..."}};
    case "Type":
      return {Info: {info: "MM"}};
    default:
      return {Error: {status: "Command not recognized."}};
  }
}

export default {
  getRedisClient: getRedisClient,
  getResponse: getResponse,
  v4: v4,
}
