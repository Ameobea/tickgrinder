//! The spawner has the secondary functionality of re-broadcasting messages received over Redis
//! through websockets and vice versa.  This allows the MM to communicate with the rest of the
//! platform while remaining a fully client-side application (can't run Redis in the browser).

use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ws::{self, WebSocket, connect, Handler};
use futures::Stream;
use serde_json::{from_str, to_string};
use uuid::Uuid;
use redis::Client as RedisClient;

use tickgrinder_util::transport::redis::{sub_all, publish, get_client as get_redis_client};
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::commands::{WrappedCommand, WrappedResponse};
use tickgrinder_util::conf::CONF;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
struct WsMsg {
    uuid: Uuid,
    channel: String,
    message: String,
}

/// Used to hold a record of which messages have been received and re-transmitted over both the WS and Redis
struct ReceivedMessages {
    hm: HashMap<WsMsg, usize>,
    first_id: usize,
    last_id: usize,
}

impl ReceivedMessages {
    /// How many UUIDs to keep a count of before dropping old values
    const MAX_SIZE: usize = 10000;

    pub fn new() -> ReceivedMessages {
        ReceivedMessages {
            hm: HashMap::new(),
            first_id: 0,
            last_id: 0,
        }
    }

    pub fn contains(&self, msg: &WsMsg) -> bool {
        self.hm.contains_key(msg)
    }

    /// Adds a new UUID to the internal `HashMap`, removing the oldest element if the size limit has been met
    pub fn add(&mut self, msg: WsMsg) {
        self.hm.insert(msg, self.last_id);
        self.last_id += 1;

        let mut oldest_msg: Option<WsMsg> = None;
        // trim old messages if the size of the collection exceeds the maximum to save memory/compute time
        if self.hm.len() > ReceivedMessages::MAX_SIZE {
            // find the UUID of the entry with id == `first_id`
            for (k, v) in self.hm.iter() {
                if *v == self.first_id {
                    oldest_msg = Some(k.clone());
                    break;
                }
            }

            if oldest_msg.is_some() {
                // remove the oldest entry from the `HashMap`
                self.hm.remove(&oldest_msg.unwrap()).unwrap();
                self.first_id += 1;
            } else {
                panic!("No entry in the `HashMap` where value == `first_id`!: {}", self.first_id);
            }
        }
    }
}

struct WsProxy {
    redis_pub_client: RedisClient,
    cs: CommandServer,
    proxied_uuids: Arc<Mutex<ReceivedMessages>>,
}

impl WsProxy {
    fn new(container: Arc<Mutex<ReceivedMessages>>, cs: CommandServer) -> WsProxy {
        WsProxy {
            redis_pub_client: get_redis_client(CONF.redis_host),
            cs: cs,
            proxied_uuids: container,
        }
    }
}

fn get_ws_host() -> String {
    format!("127.0.0.1:{}", CONF.websocket_port)
}

impl Handler for WsProxy {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        // Forard messages received on the server to Redis if they were not already sent
        let msg_string: String = match msg {
            ws::Message::Text(ref s) => s.clone(),
            ws::Message::Binary(_) => {
                self.cs.error(Some("WsMsg Parsing"), "Received binary message over websocket!");
                return Ok(());
            },
        };

        // received messages contain the destination channel and the message, so parse out of JSON first
        let res = from_str(&msg_string);
        let wsmsg = if res.is_err() {
            let errmsg = format!("Unable to parse string into `WsMsg`: {}", msg_string);
            self.cs.error(Some("WsMsg Parsing"), &errmsg);
            return Ok(());
        } else {
            res.unwrap()
        };

        // Forward the message over Redis if it hasn't already been forwarded
        let mut recvd_uuids = self.proxied_uuids.lock().unwrap();
        if !(*recvd_uuids).contains(&wsmsg) {
            recvd_uuids.add(wsmsg.clone());
            publish(&self.redis_pub_client, &wsmsg.channel, &wsmsg.message);
        }

        // self.out.broadcast(msg)

        Ok(())
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        let errmsg = format!("WebSocket connection closed with close code {:?} and reason {}", code, reason);
        self.cs.error(None, &errmsg);
        println!("{}", errmsg);
    }
}

struct WsServerHandler {
    out: ws::Sender,
    collection: Arc<Mutex<ReceivedMessages>>,
    cs: CommandServer,
}

impl WsServerHandler {
    pub fn new(out: ws::Sender, collection: Arc<Mutex<ReceivedMessages>>, cs: CommandServer) -> WsServerHandler {
        WsServerHandler {
            out: out,
            collection: collection,
            cs: cs,
        }
    }
}

impl Handler for WsServerHandler {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        // rebroadcast WS message to all connected clients if it hasn't already been rebroadcast
        let mut set = self.collection.lock().unwrap();
        let msg_string: String = match msg {
            ws::Message::Text(ref s) => s.clone(),
            ws::Message::Binary(_) => {
                self.cs.error(Some("WsMsg Parsing"), "Received binary message over websocket!");
                println!("Received binary message over websocket!");
                return Ok(());
            },
        };

        // need to get the UUID out of the JSON-encoded `WsMsg`, so parse
        let res = from_str(&msg_string);
        let wsmsg = if res.is_err() {
            let errmsg = format!("Unable to parse string into `WsMsg`: {}", msg_string);
            self.cs.error(Some("WsMsg Parsing"), &errmsg);
            println!("{}", errmsg);
            return Ok(());
        } else {
            res.unwrap()
        };

        if !set.contains(&wsmsg) {
            set.add(wsmsg);
            // re-transmit the message to all connected WS clients (this includes the sender of the message and us,
            // but we've added it to the `ReceivedMessages` object so we won't transmit again)
            self.out.broadcast(msg)
        } else {
            Ok(())
        }
    }
}

/// Proxies Redis<->Websocket traffic back and forth on new threads.  This does not block.
pub fn proxy() {
    // create a `CommandServer` for logging
    let cs = CommandServer::new(Uuid::new_v4(), "Redis<->Websocket Proxy");
    // Create a threadsafe container to hold the list of proxied messages that shouldn't be retransmitted for both the WS and Redis
    let ws_uuids = Arc::new(Mutex::new(ReceivedMessages::new()));
    let redis_uuids = Arc::new(Mutex::new(ReceivedMessages::new()));

    // spawn the websocket server, proxying messages received over it back over the WS connection to all connected clients
    let broadcaster = create_ws_server(ws_uuids.clone(), cs.clone());

    // wait a bit for the server to initialize
    thread::sleep(Duration::from_millis(50));

    // proxy messages received over Redis to the websocket
    let cs_clone_ = cs.clone();
    let redis_uuids_clone = redis_uuids.clone();
    thread::spawn(move || {
        proxy_redis(ws_uuids, redis_uuids_clone, cs_clone_, broadcaster);
    });

    // proxy messages received over WebSocket to Redis
    thread::spawn(move || {
        proxy_websocket(redis_uuids, cs);
    });
}

/// Proxies commands/responses received via Redis to Websocket
fn proxy_redis(
    ws_uuids: Arc<Mutex<ReceivedMessages>>, redis_uuids: Arc<Mutex<ReceivedMessages>>,
    mut cs: CommandServer, broadcaster: ws::Sender
) {
    // get a websocket client connected to our own websocket server
    let rx = sub_all(CONF.redis_host);

    for res in rx.wait() {
        let (chan, msg): (String, String) = res.expect("Got error in redis message loop");

        // get the UUID of the received message
        let uuid_res: Result<Uuid, ()> = if &chan == CONF.redis_control_channel || &chan == CONF.redis_log_channel {
            // parse into WrappedCommand
            let parsed_cmd: Result<WrappedCommand, ()> = match from_str::<WrappedCommand>(&msg) {
                Ok(wc) => Ok(wc),
                Err(_) => {
                    let errormsg = format!("Unable to parse message received on {} into WrappedCommand: {}", chan, msg);
                    cs.error(None, &errormsg);
                    println!("{}", errormsg);
                    Err(())
                },
            };

            match parsed_cmd {
                Ok(wrcmd) => Ok(wrcmd.uuid),
                Err(()) => Err(()),
            }
        } else {
            // parse into WrappedResponse
            let parsed_res: Result<WrappedResponse, ()> = match from_str::<WrappedResponse>(&msg) {
                Ok(wr) => Ok(wr),
                Err(_) => {
                    let errormsg = format!("Unable to parse message received on {} into WrappedResponse: {}", chan, msg);
                    cs.error(None, &errormsg);
                    println!("{}", errormsg);
                    Err(())
                },
            };

            match parsed_res {
                Ok(wrres) => Ok(wrres.uuid),
                Err(()) => Err(()),
            }
        };

        match uuid_res {
            Ok(uuid) => {
                let wsmsg = WsMsg {uuid: uuid, channel: chan.clone(), message: msg.clone()};
                let mut ws_collection = ws_uuids.lock().unwrap();
                let mut redis_collection = redis_uuids.lock().unwrap();
                if !ws_collection.contains(&wsmsg) {
                    ws_collection.add(wsmsg.clone());
                    // also add to redis collection because the fact that we're receiving it means that it was also sent
                    redis_collection.add(wsmsg);
                    let wsmsg = WsMsg {
                        uuid: uuid,
                        channel: chan,
                        message: msg.clone(),
                    };

                    let wsmsg_string: String = to_string(&wsmsg).unwrap();
                    broadcaster.broadcast(wsmsg_string.as_str()).expect("Unable to send message over websocket");
                }
            },
            Err(()) => {
                cs.error(None, &format!("Unable to get uuid from message: {}", msg));
            }
        }
    }
}

/// Proxy all messages received over the websocket server to Redis
fn proxy_websocket(container: Arc<Mutex<ReceivedMessages>>, cs: CommandServer) {
    connect(format!("ws://{}", get_ws_host()), |_| {
        WsProxy::new(container.clone(), cs.clone())
    }).expect("Unable to initialize websocket proxy");
}

/// Starts a websocket server used to proxy the messages.  Returns a `ws::Sender` that can be used to broadcast messages
/// to all connected clients of the server.
fn create_ws_server(collection: Arc<Mutex<ReceivedMessages>>, cs: CommandServer) -> ws::Sender {
    let server = WebSocket::new(move |out: ws::Sender| {
        let collection_clone = collection.clone();
        let cs_clone = cs.clone();
        WsServerHandler::new(out, collection_clone, cs_clone)
    }).expect("Unable to initialize websocket server!");

    let broadcaster = server.broadcaster();

    thread::spawn(move || {
        // start the server on a separate thread
        server.listen(get_ws_host()).expect("Unable to initialize websocket server!");
    });

    broadcaster
}
