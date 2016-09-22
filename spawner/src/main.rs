//! Bot4 Instance Spawner and Manager
//!
//! Responsible for spawning, destroying, and managing all instances of the bot4
//! platform's modules and reporting on their status.

extern crate uuid;
extern crate algobot_util;

mod conf;

use std::sync::Mutex;

use conf::CONF;

use uuid::Uuid;
use algobot_util::transport::redis::sub_channel;

/// Represents an instance of a platform module.  Contains a Uuid to identify it
/// as well as some information about its spawning parameters and its type.
struct Instance {
    // TODO: Spawning parameters
    instance_type: String,
    uuid: Uuid
}

/// Holds a list of all instances that the spawner has spawned and thinks are alive
struct InstanceManager {
    living: Mutex<Vec<Instance>>
}

impl InstanceManager {
    /// Creates a new spawner instance.
    pub fn new() -> InstanceManager {
        InstanceManager {
            living: Mutex::new(Vec::new())
        }
    }

    /// Starts listening for commands on the control channel, spawns a new MM instance,
    /// and initializes the ping heartbeat.
    pub fn init(&mut self) {
        // listen for new commands and setup callbacks
        self.listen();
        // spawn a MM instance
        self.spawn_mm();
        // start ping heartbeat
        loop {
            let res = self.ping_all();
        }
    }

    /// Starts listening for new commands on the control channel
    pub fn listen(&mut self) {
        // sub to spawer control channel
        let cmds_rx = sub_channel(CONF.redis_url, CONF.redis_control_channel);
        // TODO
    }

    /// Spawns a new MM server instance and inserts its Uuid into the living instances list
    fn spawn_mm(&mut self) {
        unimplemented!();
    }

    /// Spawns a new Tick Processor instance with the given symbol andinserts its Uuid into
    /// the living instances list
    fn spawn_tick_processor(&mut self, symbol: String) {
        unimplemented!();
    }

    /// Spawns a new Optimizer instance with the specified strategy andinserts its Uuid into
    /// the living instances list
    fn spawn_optimizer(&mut self, strategy: String) {
        unimplemented!();
    }

    /// Sends a Ping command to all instances that the spawner thinks are running.  After
    /// 1 second.  If an instance is nonresponsive after 5 retries, it is assumed to be
    /// dead and is respawned.  Also sends a message to the logger when this happens.
    ///
    /// Returns Some([dead_instance, ...]) or None
    fn ping_all(&mut self) -> Option<Vec<Instance>> {
        unimplemented!();
    }

}

fn main() {
    let mut spawner = InstanceManager::new();
    spawner.init();
}
