//! Functions for communication with the platform's modules using Redis pub/sub.

// var redis = require("redis");
// TODO: Redis<->Websocket shim service running between the GUI and the platform.
// write it in Rust?
const CONF = require("../conf.js");

import { getRedisClient, getResponse } from '../utils/redis';

export default {
  namespace: 'redis',

  state: {},

  reducers: {
    commandReceived(state = {}, action) {
      // TODO
    },

    responseReceived(state = {}, action) {
      // TODO
    },

    logReceived(state = {}, action) {
      // TODO
    },
  },

  subscriptions: {
    redisListener({ dispatch }) {
      // initialize redis clients for sending and receiving messages
      let subClient = getRedisClient();
      let pubClient = getRedisClient();

      // subscribe to command, response, and log channels
      subClient.subscribe(CONF.redis_control_channel);
      subClient.subscribe(CONF.redis_responses_channel);
      subClient.subscribe(CONF.redis_log_channel);

      subClient.on("message", (channel, message_str)=>{
        // convert the {"Enum"}s to plain strings
        message_str = message_str.replace(/{("\w*")}/g, "$1");
        var wr_msg = JSON.parse(message_str);
        if(wr_msg.cmd && !wr_msg.cmd.Log){
          var response = getResponse(wr_msg.cmd);
          var wr_res = {uuid: wr_msg.uuid, res: response};
          pubClient.publish(conf.redis_responses_channel, JSON.stringify(wr_res));
        }
      });
    }
  },
}
