var conf = {
  // MM Config
  mmUrl: "localhost",
  mmPort: 8002,
  // Websockets Config
  websocketUrl: "localhost",
  websocketPort: 7037,
  // Postgres Config
  postgresUrl: "localhost",
  postgresPort: 5432,
  postgresUser: "user",
  postgresPassword: "password",
  postgresDatabase: "algobot",
  // Redis Config
  redisUrl: "localhost",
  redisPort: 6379,
  redisCommandsChannel: "commands",
  redisResponsesChannel: "responses"
  // General Config
  tickDataDirectory: "/home/casey/bot4/data/historical_ticks/"
};

module.exports = conf;
