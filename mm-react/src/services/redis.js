//! Functions for interfacing with the platform's Redis-based communication system

const CONF = require("../conf.js");

/// Returns a new Redis client based on the settings in conf
function getRedisClient() {
  var spl = conf.redis_host.split("://")[1].split(":");
  return redis.createClient({
    host: spl[0],
    port: parseInt(spl[1]),
  });
}

