//! Defines the `PlatformInstance` trait which can be implemented to create instances that communicate on the platform,
//! sending/receiving commands and interacting with other instances.

use std::str::FromStr;

use uuid::Uuid;
use futures::Stream;
use redis;
use serde_json;

use transport::commands::{Command, Response, WrappedCommand, send_command};
use transport::command_server::CommandServer;
use transport::redis::{get_client, sub_multiple};
use conf::CONF;

pub trait PlatformInstance {
    /// Instructs the instance to start listening for and responding to commands.
    fn listen(mut self, uuid: Uuid, cs: &mut CommandServer) where Self:Sized {
        // subscribe to the command channels
        let rx = sub_multiple(
            CONF.redis_host, &[CONF.redis_control_channel, uuid.hyphenated().to_string().as_str()]
        );
        let redis_client = get_client(CONF.redis_host);

        // Signal to the platform that we're ready to receive commands
        let _ = send_command(&WrappedCommand::from_command(
            Command::Ready{instance_type: "Backtester".to_string(), uuid: uuid}), &redis_client, "control"
        );

        for res in rx.wait() {
            let (_, msg) = res.expect("Received err in the listen() event loop for the backtester!");
            let wr_cmd = match WrappedCommand::from_str(msg.as_str()) {
                Ok(wr) => wr,
                Err(e) => {
                    cs.error(Some("CommandProcessing"), &format!("Unable to parse WrappedCommand from String: {:?}", e));
                    return;
                }
            };

            let res: Option<Response> = self.handle_command(wr_cmd.cmd);
            if res.is_some() {
                redis::cmd("PUBLISH")
                    .arg(CONF.redis_responses_channel)
                    .arg(&res.unwrap().wrap(wr_cmd.uuid).to_string().unwrap())
                    .execute(&redis_client);
                }
        }
    }

    /// Given a `Command` from the platform, process it and optionally return a `Response` to be sent as a reply.
    fn handle_command(&mut self, cmd: Command) -> Option<Response>;
}
