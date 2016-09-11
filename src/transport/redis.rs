//! Functions for interfacing with Redis

use std::thread;
use std::str::FromStr;

use redis;
use futures::Future;
use futures::stream::{channel, Sender, Receiver};

pub fn get_client(host: &str) -> redis::Client {
    redis::Client::open(host).expect("Could not connect to redis")
}

/// Blocks until a message is received on a pubsub then returns it
/// Returns the message as a String
fn get_message(ps: &redis::PubSub) -> String {
    let msg = ps.get_message().expect("Could not get message from pubsub!");
    msg.get_payload::<String>().expect("Could not convert redis message to string!")
}

/// Recursively call get_message and send the results over tx
fn get_message_outer(tx: Sender<String, ()>, ps: &redis::PubSub) -> Result<Sender<String, ()>, String> {
    // block until a new message is received
    let res = get_message(ps);
    // block again until the message is consumed
    // this prevents the tx from dropping since .send() is async
    tx.send(Ok(res)).wait()
        .map_err(|_| String::from_str("Error while trying to get new tx from .send in redis.rs").unwrap() )
}

pub fn get_pubsub(host: &str, channel: &'static str) -> redis::PubSub {
    let client = get_client(host);
    let mut pubsub = client.get_pubsub()
        .expect("Could not create pubsub for redis client");
    pubsub.subscribe(channel)
        .expect("Could not subscribe to pubsub channel");
    pubsub
}

/// Returns a Receiver that resolves to new messages received
/// on a pubsub channel
pub fn sub_channel(host: &str, ps_channel: &'static str) -> Receiver<String, ()> {
    let (tx, rx) = channel::<String, ()>();
    let ps = get_pubsub(host, ps_channel);
    let mut new_tx = tx;
    thread::spawn(move || {
        while let Ok(_new_tx) = get_message_outer(new_tx, &ps) {
            new_tx = _new_tx;
        }
        println!("Channel subscription expired");
    });
    rx
}
