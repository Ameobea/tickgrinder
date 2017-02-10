//! The spawner has the secondary functionality of re-broadcasting messages received over Redis
//! through websockets and vice versa.  This allows the MM to communicate with the rest of the
//! platform while remaining a fully client-side application (can't run Redis in the browser).

use std::thread;
use std::time::Duration;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use ws::{self, WebSocket, connect, Handler};
use futures::Stream;
use serde_json::{from_str, to_string};
use uuid::Uuid;
use redis::Client as RedisClient;

use tickgrinder_util::transport::redis::{sub_multiple, publish, get_client as get_redis_client};
use tickgrinder_util::transport::command_server::CommandServer;
use tickgrinder_util::transport::commands::{WrappedCommand, WrappedResponse};
use tickgrinder_util::conf::CONF;

#[derive(Serialize, Deserialize)]
struct WsMsg {
    uuid: Uuid,
    channel: String,
    message: String,
}

struct WsProxy {
    out: ws::Sender,
    redis_pub_client: RedisClient,
    cs: CommandServer,
    proxied_uuids: Arc<Mutex<VecDeque<String>>>,
}

impl WsProxy {
    fn new(out: ws::Sender, container: Arc<Mutex<VecDeque<String>>>, cs: CommandServer) -> WsProxy {
        WsProxy {
            out: out,
            redis_pub_client: get_redis_client(CONF.redis_host),
            cs: cs,
            proxied_uuids: container,
        }
    }
}

impl Handler for WsProxy {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        // Forard messages received on the server to Redis if they were not already sent
        let msg_string: String = match msg {
            ws::Message::Text(ref s) => s.clone(),
            ws::Message::Binary(_) => {
                self.cs.error(Some("WsMsg Parsing"), "Received binary message over websocket!");
                println!("Received binary message over websocket!");
                return Ok(());
            },
        };

        // received messages contain the destination channel and the message, so parse out of JSON first
        let res = from_str(&msg_string);
        let WsMsg{uuid: _, channel, message: wrapped_msg_string} = if res.is_err() {
            let errmsg = format!("Unable to parse string into `WsMsg`: {}", msg_string);
            self.cs.error(Some("WsMsg Parsing"), &errmsg);
            println!("{}", errmsg);
            return Ok(());
        } else {
            res.unwrap()
        };

        // Forward the message over Redis if it hasn't already been forwarded
        let mut set = self.proxied_uuids.lock().unwrap();
        if !set.contains(&wrapped_msg_string) {
            set.push_back(wrapped_msg_string.clone());
            if set.len() > 25000 {
                let _ = set.pop_front();
            }
            publish(&self.redis_pub_client, &channel, &wrapped_msg_string);
        }

        self.out.broadcast(msg)
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        let errmsg = format!("WebSocket connection closed with close code {:?} and reason {}", code, reason);
        self.cs.error(None, &errmsg);
        println!("{}", errmsg);
    }
}

struct WsServerHandler {
    out: ws::Sender,
    collection: Arc<Mutex<VecDeque<String>>>,
    cs: CommandServer,
}

impl WsServerHandler {
    pub fn new(out: ws::Sender, collection: Arc<Mutex<VecDeque<String>>>, cs: CommandServer) -> WsServerHandler {
        WsServerHandler {
            out: out,
            collection: collection,
            cs: cs,
        }
    }
}

impl Handler for WsServerHandler {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        // rebroadcast message to all connected clients if it hasn't already been rebroadcast
        let mut set = self.collection.lock().unwrap();
        let msg_string: String = match msg {
            ws::Message::Text(ref s) => s.clone(),
            ws::Message::Binary(_) => {
                self.cs.error(Some("WsMsg Parsing"), "Received binary message over websocket!");
                println!("Received binary message over websocket!");
                return Ok(());
            },
        };

        if !set.contains(&msg_string) {
            set.push_back(msg_string.clone());
            let _ = set.pop_front();
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
    // Create a threadsafe container to hold the list of proxied messages that shouldn't be retransmitted
    let container = Arc::new(Mutex::new(VecDeque::new()));

    // spawn the websocket server, proxying messages received over it to Redis
    let container_clone = container.clone();
    let cs_clone = cs.clone();
    let broadcaster = create_ws_server(container_clone, cs_clone);

    // wait a bit for the server to initialize
    thread::sleep(Duration::from_millis(50));

    // proxy messages received over Redis to the websocket
    let container_clone_ = container.clone();
    let cs_clone_ = cs.clone();
    thread::spawn(move || {
        proxy_redis(container_clone_, cs_clone_, broadcaster);
    });

    // proxy messages received over WebSocket to Redis
    thread::spawn(move || {
        proxy_websocket(container, cs);
    });
}

/// Proxies commands/responses received via Redis to Websocket
fn proxy_redis(container: Arc<Mutex<VecDeque<String>>>, mut cs: CommandServer, broadcaster: ws::Sender) {
    // get a websocket client connected to our own websocket server
    let rx = sub_multiple(
        CONF.redis_host, &[CONF.redis_control_channel, CONF.redis_responses_channel, CONF.redis_log_channel]
    );

    for res in rx.wait() {
        let (chan, msg) = res.expect("Got error in redis message loop");

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
                let mut collection = container.lock().unwrap();
                if !collection.contains(&msg) {
                    collection.push_back(msg.clone());
                    let wsmsg = WsMsg {
                        uuid: uuid,
                        channel: chan,
                        message: msg,
                    };
                    if collection.len() > 25000 {
                        let _ = collection.pop_front();
                    }

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
fn proxy_websocket(container: Arc<Mutex<VecDeque<String>>>, cs: CommandServer) {
    connect(format!("ws://{}", get_ws_host()), |out| {
        WsProxy::new(out, container.clone(), cs.clone())
    }).expect("Unable to initialize websocket proxy");
}

fn get_ws_host() -> String {
    format!("127.0.0.1:{}", CONF.websocket_port)
}

/// Starts a websocket server used to proxy the messages.  Returns a `ws::Sender` that can be used to broadcast messages
/// to all connected clients of the server.
fn create_ws_server(collection: Arc<Mutex<VecDeque<String>>>, cs: CommandServer) -> ws::Sender {
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
