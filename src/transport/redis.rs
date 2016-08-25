// Responsible for receiving live ticks and other kinds of data and
// sending calculations and results back.

use std::thread;

use redis;
use futures::Future;
use futures::stream::{channel, Receiver, Sender};

use conf::CONF;

pub fn get_client() -> redis::Client {
    redis::Client::open(CONF.redis_url).expect("Could not connect to redis")
}

fn get_pubsub(channel: &'static str) -> redis::PubSub {
    let client = get_client();
    let mut pubsub = client.get_pubsub()
        .expect("Could not create pubsub for redis client");
    pubsub.subscribe(channel)
        .expect("Could not subscribe to pubsub channel");
    pubsub
}

// Blocks until a message is received on a pubsub then returns it
// Returns the message as a String
fn get_message(ps: &redis::PubSub) -> String {
    let msg = ps.get_message().expect("Could not get message from pubsub!");
    msg.get_payload::<String>().expect("Could not convert redis message to string!")
}

// Recursively call get_message and send the results over tx
fn get_message_outer(tx: Sender<String, ()>, ps: &redis::PubSub) {
    // block until a new message is received
    let res = get_message(ps);
    // block again until the message is consumed
    // this prevents the tx from dropping since .send() is async
    tx.send(Ok(res)).wait().map(|new_tx| {
        // start listening again
        get_message_outer(new_tx, ps);
    });
}

// Returns a Receiver that resolves to new messages received
// on a pubsub channel
pub fn sub_channel(ps_channel: &'static str) -> Receiver<String, ()> {
    let (tx, rx) = channel::<String, ()>();
    let ps = get_pubsub(ps_channel);
    thread::spawn(move || {
        get_message_outer(tx, &ps);
    });
    rx
}
