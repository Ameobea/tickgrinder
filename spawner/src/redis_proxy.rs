//! The spawner has the secondary functionality of re-broadcasting messages received over Redis
//! through websockets and vice versa.  This allows the MM to communicate with the rest of the
//! platform while remaining a fully client-side application (can't run Redis in the browser).

use std::thread;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use ws::{self, connect, listen, Handler};
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

struct WsServer {
    out: ws::Sender,
    redis_pub_client: RedisClient,
    cs: CommandServer,
    proxied_uuids: Arc<Mutex<HashSet<Uuid>>>,
}

impl WsServer {
    fn new(out: ws::Sender, container: Arc<Mutex<HashSet<Uuid>>>, cs: CommandServer) -> WsServer {
        WsServer {
            out: out,
            redis_pub_client: get_redis_client(CONF.redis_host),
            cs: cs,
            proxied_uuids: container,
        }
    }
}

struct WsClient {
    rx_iter: Box<Iterator<Item=Result<(String, String), ()>>>,
    out: ws::Sender,
    cs: CommandServer,
    proxied_uuids: Arc<Mutex<HashSet<Uuid>>>,
}

impl WsClient {
    fn new(out: ws::Sender, container: Arc<Mutex<HashSet<Uuid>>>, cs: CommandServer) -> WsClient {
        // subscribe to messages on commands, responses, and logging channels
        let rx = sub_multiple(
            CONF.redis_host, &[CONF.redis_control_channel, CONF.redis_responses_channel, CONF.redis_log_channel]
        );

        WsClient {
            rx_iter: Box::new(rx.wait()),
            out: out,
            cs: cs,
            proxied_uuids: container,
        }
    }

    fn handle_redis_msg(&mut self, chan: String, msg: String) {
        println!("Received message on Redis on channel {}: {}", chan, msg);
        // create a WsMsg out of the received Redis message
        let uuid_res: Result<Uuid, ()> = if &chan == CONF.redis_control_channel || &chan == CONF.redis_log_channel { // parse into WrappedCommand
            let parsed_cmd: Result<WrappedCommand, ()> = match from_str::<WrappedCommand>(&msg) {
                Ok(wc) => Ok(wc),
                Err(_) => {
                    let errormsg = format!("Unable to parse message received on {} into WrappedCommand: {}", chan, msg);
                    self.cs.error(None, &errormsg);
                    println!("{}", errormsg);
                    Err(())
                },
            };

            match parsed_cmd {
                Ok(wrcmd) => Ok(wrcmd.uuid),
                Err(()) => Err(()),
            }
        } else { // parse into WrappedResponse
            let parsed_res: Result<WrappedResponse, ()> = match from_str::<WrappedResponse>(&msg) {
                Ok(wr) => Ok(wr),
                Err(_) => {
                    let errormsg = format!("Unable to parse message received on {} into WrappedResponse: {}", chan, msg);
                    self.cs.error(None, &errormsg);
                    println!("{}", errormsg);
                    Err(())
                },
            };

            match parsed_res {
                Ok(wrres) => Ok(wrres.uuid),
                Err(()) => Err(()),
            }
        };

        if uuid_res.is_ok() {
            let uuid = uuid_res.unwrap();

            // only send message if it hasn't already been proxied.
            let mut set = self.proxied_uuids.lock().unwrap();
            // if !set.contains(&uuid) {
                let wsmsg = WsMsg {
                    channel: chan,
                    uuid: uuid,
                    message: msg,
                };

                set.insert(uuid);

                let wsmsg_string: String = to_string(&wsmsg).unwrap();
                println!("Sending message over websocket...");
                self.out.broadcast(wsmsg_string.as_str()).expect("Unable to send message over websocket");
            // }
        }
    }
}

impl Handler for WsClient {
    fn on_open(&mut self, _: ws::Handshake) -> Result<(), ws::Error> {
        let first_msg = self.rx_iter.next().unwrap();
        let (chan, msg) = first_msg.expect("Got error in the `WsClient` redis rx wait loop)");
        self.handle_redis_msg(chan, msg);

        Ok(())
    }

    fn on_message(&mut self, _: ws::Message) -> Result<(), ws::Error> {
        // It's not out job to forward messages to Redis (we would do it poorly because we're blocking)
        // so just block and wait for the next Redis message and send it which triggers this to be called again.
        let (chan, msg) = self.rx_iter.next().unwrap().expect("Got error in the `WsClient` redis rx wait loop)");
        self.handle_redis_msg(chan, msg);

        Ok(())
    }
}

impl Handler for WsServer {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        // Forard messages received on the server to Redis if they were not already sent
        let msg_string: String = match msg {
            ws::Message::Text(ref s) => s.clone(),
            ws::Message::Binary(_) => {
                // self.cs.error(Some("WsMsg Parsing"), "Received binary message over websocket!");
                println!("Received binary message over websocket!");
                return Ok(());
            },
        };

        println!("Received message on websocket: {}", msg_string);

        // received messages contain the destination channel and the message, so parse out of JSON first
        let res = from_str(&msg_string);
        let WsMsg{uuid, channel, message: _} = if res.is_err() {
            let errmsg = format!("Unable to parse string into `WsMsg`: {}", msg_string);
            // self.cs.error(Some("WsMsg Parsing"), &errmsg);
            println!("{}", errmsg);
            return Ok(());
        } else {
            res.unwrap()
        };

        // Forward the message over Redis if it hasn't already been forwarded
        let mut set = self.proxied_uuids.lock().unwrap();
        if !set.contains(&uuid) {
            set.insert(uuid);
            publish(&self.redis_pub_client, &channel, &msg_string);
        }

        self.out.broadcast(msg)
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        let errmsg = format!("WebSocket connection closed with close code {:?} and reason {}", code, reason);
        self.cs.error(None, &errmsg);
        println!("{}", errmsg);
    }
}

/// Proxies Redis<->Websocket traffic back and forth on new threads.  This does not block.
pub fn proxy() {
    // create a `CommandServer` for logging
    let cs = CommandServer::new(Uuid::new_v4(), "Redis<->Websocket Proxy");
    // Create a threadsafe container to hold the list of proxied messages that shouldn't be retransmitted
    let container = Arc::new(Mutex::new(HashSet::new()));

    // spawn the websocket server, proxying messages received over it to Redis
    create_ws_server(container.clone(), cs.clone());

    // proxy messages received over Redis to the websocket
    let container_clone = container.clone();
    thread::spawn(move || {
        proxy_redis(container_clone, cs);
    });
}

/// Proxies commands/responses received via Redis to Websocket
fn proxy_redis(container: Arc<Mutex<HashSet<Uuid>>>, cs: CommandServer) {
    // get a websocket client connected to our own websocket server
    connect(format!("ws://{}", get_ws_host()), move |out| { // Move out @dalexj
        WsClient::new(out, container.clone(), cs.clone())
    }).expect("Unable to create Redis<->Websocket Proxy");
}

fn get_ws_host() -> String {
    format!("127.0.0.1:{}", CONF.websocket_port)
}

/// Starts a websocket server used to proxy the messages.  Returns a oneshot that fulfills once the
/// server has been started.
fn create_ws_server(container: Arc<Mutex<HashSet<Uuid>>>, cs: CommandServer) {
    thread::spawn(move || {
        listen(get_ws_host(), move |out| {
            WsServer::new(out, container.clone(), cs.clone())
        }).expect("Unable to start websocket server!");
    });
}
