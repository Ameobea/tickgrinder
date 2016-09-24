//! Bot4 Instance Spawner and Manager
//!
//! Responsible for spawning, destroying, and managing all instances of the bot4
//! platform's modules and reporting on their status.

#![feature(test)]

extern crate uuid;
extern crate redis;
extern crate algobot_util;
extern crate futures;
extern crate test;

mod conf;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::process;

use conf::CONF;

use uuid::Uuid;
use futures::{Future, oneshot, Oneshot, Complete};
use futures::stream::Stream;
use algobot_util::transport::redis::{sub_channel, get_client};
use algobot_util::transport::commands::*;
use algobot_util::transport::command_server::*;

/// Represents an instance of a platform module.  Contains a Uuid to identify it
/// as well as some information about its spawning parameters and its type.
#[derive(Debug)]
struct Instance {
    // TODO: Spawning parameters
    instance_type: String,
    uuid: Uuid
}

/// Holds a list of all instances that the spawner has spawned and thinks are alive
#[derive(Clone)]
struct InstanceManager {
    pub uuid: Uuid,
    pub living: Arc<Mutex<Vec<Instance>>>
}

impl InstanceManager {
    /// Creates a new spawner instance.
    pub fn new() -> InstanceManager {
        InstanceManager {
            uuid: Uuid::new_v4(),
            living: Arc::new(Mutex::new(Vec::new()))
        }
    }

    /// Starts listening for commands on the control channel, spawns a new MM instance,
    /// and initializes the ping heartbeat.
    pub fn init(&mut self) {
        // Look for old running instances and either take control of them or kill them depending on conf
        // TODO
        // listen for new commands and setup callbacks
        self.listen();
        // spawn a MM instance
        self.spawn_mm();
        // start ping heartbeat
        loop {
            // blocks until all instances return their expected responses or time out
            let res = self.ping_all();
            match res {
                Some(dead_instances) => {
                    println!("Dead instances: {:?}", dead_instances);
                },
                None => {},
            }

            thread::sleep(Duration::new(1,0));
        }
    }

    /// Starts listening for new commands on the control channel
    pub fn listen(&mut self) {
        let mut dup = self.clone();

        thread::spawn(move || {
            // sub to spawer control channel
            let cmds_rx = sub_channel(CONF.redis_url, CONF.redis_control_channel);
            let mut redis_client = get_client(CONF.redis_url);

            cmds_rx.for_each(move |cmd_string| {
                match WrappedCommand::from_str(cmd_string.as_str()) {
                    Ok(wr_cmd) => {
                        let (c, o) = oneshot::<Response>();
                        dup.handle_command(wr_cmd.cmd, c);

                        let uuid = wr_cmd.uuid.clone();
                        o.and_then(|status: Response| {
                            redis::cmd("PUBLISH")
                                .arg(CONF.redis_responses_channel)
                                .arg(status.wrap(uuid).to_string().unwrap().as_str())
                                .execute(&mut redis_client);
                            Ok(())
                        }).wait();
                    },
                    Err(_) => {
                        println!("Couldn't parse WrappedCommand from: {:?}", cmd_string);
                    },
                }

                Ok(())
            }).wait();
        });
    }

    /// Processes an incoming command, doing whatever it instructs and fulfills the future
    /// that it fulfills with the status once it's finished.
    fn handle_command(&mut self, cmd: Command, c: Complete<Response>) {
        let res = match cmd {
            Command::Ping => Response::Pong{uuid: self.uuid},
            Command::KillAllInstances => self.kill_all(),
            _ => Response::Error{
                status: "Command not accepted by the instance spawner.".to_string()
            }
        };
        c.complete(res);
    }

    /// Spawns a new MM server instance and inserts its Uuid into the living instances list
    fn spawn_mm(&mut self) {
        let mod_uuid = Uuid::new_v4();
        let _ = process::Command::new(CONF.node_binary_path)
                                .arg("../mm/manager.js")
                                .arg(mod_uuid.to_string().as_str())
                                .spawn()
                                .expect("Unable to spawn MM");
        self.add_instance(Instance{instance_type: "MM".to_string(), uuid: mod_uuid});
    }

    /// Spawns a new Tick Processor instance with the given symbol andinserts its Uuid into
    /// the living instances list
    fn spawn_tick_parser(&mut self, symbol: String) {
        let mod_uuid = Uuid::new_v4();
        let _ = process::Command::new("../tick_parser/target/debug/tick_processor")
                                .arg(mod_uuid.to_string().as_str())
                                .arg(symbol.as_str())
                                .spawn()
                                .expect("Unable to spawn Tick Parser");
        self.add_instance(Instance{instance_type: "Tick Parser".to_string(), uuid: mod_uuid});
    }

    /// Spawns a new Optimizer instance with the specified strategy andinserts its Uuid into
    /// the living instances list
    fn spawn_optimizer(&mut self, strategy: String) {
        let mod_uuid = Uuid::new_v4();
        let _ = process::Command::new("../optimizer/target/debug/optimizer")
                                .arg(mod_uuid.to_string().as_str())
                                .arg(strategy.as_str())
                                .spawn()
                                .expect("Unable to spawn Optimizer");
        self.add_instance(Instance{instance_type: "Optimizer".to_string(), uuid: mod_uuid});
    }

    /// Sends a Ping command to all instances that the spawner thinks are running.  After
    /// 1 second.  If an instance is nonresponsive after 5 retries, it is assumed to be
    /// dead and is respawned.  Also sends a message to the logger when this happens.
    ///
    /// Returns Some([dead_instance, ...]) or None
    fn ping_all(&mut self) -> Option<Vec<Instance>> {
        unimplemented!();
    }

    /// Kills all currently running instances managed by this spawner
    fn kill_all(&mut self) -> Response {

    }

    /// Adds an instance to the internal living instances list
    fn add_instance(&mut self, inst: Instance) {
        let l = self.living.clone();
        let mut ll = l.lock().unwrap();
        ll.push(inst);
    }

    /// Removes an instance with the given Uuid from the internal instances list
    fn remove_instance(&mut self, uuid: Uuid) {
        let l = self.living.clone();
        let mut ll = l.lock().unwrap();

        let mut _i: Option<usize> = None;
        for (i, inst) in (*ll).iter().enumerate() {
            if inst.uuid == uuid {
                _i = Some(i);
            }
        }

        if _i.is_some() {
            ll.remove(_i.unwrap());
        }
    }
}

fn main() {
    let mut spawner = InstanceManager::new();
    spawner.init();
}

/// Tests the instance manager's ability to process incoming Commands.
#[test]
fn spawner_command_processing() {
    let mut spawner = InstanceManager::new();
    spawner.listen();

    let mut client = get_client(CONF.redis_url);
    let cmd = Command::Ping.wrap();
    let cmd_string = cmd.to_string().unwrap();

    let rx = sub_channel(CONF.redis_url, CONF.redis_responses_channel);
    // send a Ping command
    redis::cmd("PUBLISH")
        .arg(CONF.redis_control_channel)
        .arg(cmd_string.as_str())
        .execute(&mut client);

    // Wait for a Pong to be received
    let res = rx.wait().next().unwrap().unwrap();
    assert_eq!(WrappedResponse::from_str(res.as_str()).unwrap().res, Response::Pong{uuid: spawner.uuid});
}

#[test]
fn tick_processor_spawning() {
    let mut spawner = InstanceManager::new();
    spawner.spawn_tick_parser("_test3".to_string());

    let living = spawner.living.clone();
    let living_inner = living.lock().unwrap();
    assert_eq!((*living_inner).len(), 1);
}
