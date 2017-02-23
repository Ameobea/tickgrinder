//! Contains the definitions of all commands in the Command intermodular communication
//! system as well as helper functions for Serialization/Deserialization and unwrapping.

use std::str::FromStr;

use serde_json;
use redis;
use uuid::Uuid;
#[allow(unused_imports)]
use test;

use std::collections::HashMap;

/// Represents a Command that can be serde'd and sent over Redis.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Command {
    // Generic Commands; all instances must implement responses for these commands.
    Ping,
    Shutdown,
    Kill,
    Register {channel: String},
    Type, // returns what kind of instance this is
    Ready {instance_type: String, uuid: Uuid}, /* signals that a newly spawned instance is ready to receive commands */
    // Tick Processor Commands
    AddCondition {condition_string: String},
    RemoveCondition {condition_string: String},
    ListConditions,
    SubTicks {broker_def: String},
    // Spawner Commands
    Census,
    SpawnOptimizer{strategy: String},
    SpawnTickParser{symbol: String},
    SpawnBacktester,
    SpawnLogger,
    SpawnFxcmNativeDataDownloader,
    SpawnFxcmFlatfileDataDownloader,
    KillInstance{uuid: Uuid},
    KillAllInstances,
    // Commands for interfacing with the document store
    QueryDocumentStore{query: String},
    InsertIntoDocumentStore{doc: String},
    GetDocument{title: String},
    // Backtester Commands
    StartBacktest{definition: String},
    PauseBacktest{uuid: Uuid},
    ResumeBacktest{uuid: Uuid},
    StopBacktest{uuid: Uuid},
    ListBacktests,
    ListSimbrokers,
    SpawnSimbroker{settings: HashMap<String, String>},
    // Data Downloader Commands
    // TODO: Create a `DataDownload` struct and replace these with that
    DownloadTicks {
        start_time: u64,
        end_time: u64,
        symbol: String,
        dst: HistTickDst,
    },
    ListRunningDownloads,
    DownloadComplete {
        id: Uuid,
        downloader: Instance, // the `Instance` managing this download
        start_time: u64,
        end_time: u64,
        symbol: String,
        dst: HistTickDst,
    },
    DownloadStarted {
        id: Uuid,
        downloader: Instance, // the `Instance` managing this download
        start_time: u64,
        end_time: u64,
        symbol: String,
        dst: HistTickDst,
    },
    GetDownloadProgress {id: Uuid},
    CancelDataDownload{download_id: Uuid},
    TransferHistData { src: HistTickDst, dst: HistTickDst },
    // Logger Commands
    Log { msg: LogMessage },
}

/// Represents a response from the Tick Processor to a Command sent
/// to it at some earlier point.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Response {
    // Generic Responses
    Ok,
    Error{status: String},
    Pong{args: Vec<String>},
    Info{info: String},
    DocumentQueryResult{results: Vec<String>},
    Document{doc: SrcDocument},
    DownloadProgress{
        id: Uuid,
        start_time: u64,
        cur_time: u64,
        end_time: u64,
    },
}

impl Command {
    pub fn to_string(&self) -> Result<String, ()> {
        serde_json::to_string(self).map_err(|_| ())
    }

    /// Generates a new Uuid and creates a new WrappedCommand
    pub fn wrap(&self) -> WrappedCommand {
        WrappedCommand {
            uuid: Uuid::new_v4(),
            cmd: self.clone(),
        }
    }
}

impl FromStr for Command {
    type Err = ();

    fn from_str(raw: &str) -> Result<Command, ()> {
        serde_json::from_str(raw).map_err(|_| ())
    }
}

/// Where to save the recorded ticks to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HistTickDst {
    Flatfile { filename: String },
    Postgres { table: String },
    RedisChannel { host: String, channel: String },
    RedisSet { host: String, set_name: String },
    Console,
}

/// A log message from some part of the platform
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogMessage {
    pub sender: Instance,
    pub message_type: String,
    pub message: String,
    pub level: LogLevel,
}

/// Represents an instance of a platform module.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Instance {
    pub instance_type: String,
    pub uuid: Uuid,
}

/// Severity of a log message, Notice through Critical
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum LogLevel {
    Debug,
    Notice,
    Warning,
    Error,
    Critical,
}

/// C-format `LogLevel` enum for use in FFI
#[repr(C)]
#[allow(dead_code)]
pub enum CLogLevel {
    DEBUG,
    NOTICE,
    WARNING,
    ERR,
    CRITICAL,
}

impl CLogLevel {
    #[allow(dead_code)]
    pub fn convert(&self) -> LogLevel {
        match *self {
            CLogLevel::DEBUG => LogLevel::Debug,
            CLogLevel::NOTICE => LogLevel::Notice,
            CLogLevel::WARNING => LogLevel::Warning,
            CLogLevel::ERR => LogLevel::Error,
            CLogLevel::CRITICAL => LogLevel::Critical,
        }
    }
}

/// A document that can be stored in the Tantivy document database.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SrcDocument {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub creation_date: String,
    pub modification_date: String,
    pub id: Uuid,
}

/// Represents a command bound to a unique identifier that can be
/// used to link it with a Response
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct WrappedCommand {
    pub uuid: Uuid,
    pub cmd: Command,
}

impl WrappedCommand {
    pub fn to_string(&self) -> Result<String, ()> {
        serde_json::to_string(self).map_err(|_| ())
    }

    /// Creates a new WrappedCommand with the given command as an inner
    pub fn from_command(cmd: Command) -> WrappedCommand {
        WrappedCommand {
            uuid: Uuid::new_v4(),
            cmd: cmd.clone(),
        }
    }
}

impl FromStr for WrappedCommand {
    type Err = ();

    fn from_str(raw: &str) -> Result<WrappedCommand, ()> {
        serde_json::from_str(raw).map_err(|_| ())
    }
}

#[allow(doc_markdown)]
/// Converts a String into a `WrappedCommand`
/// JSON Format: {"uuid": "xxxx-xxxx", "cmd": {"CommandName":{"arg": "val"}}}
pub fn parse_wrapped_command(raw: String) -> WrappedCommand {
    let res = serde_json::from_str::<WrappedCommand>(&raw);
    match res {
        Ok(wr_cmd) => wr_cmd,
        Err(_) => panic!("Unable to parse WrappedCommand from String: {}", raw),
    }
}

impl Response {
    pub fn to_string(&self) -> Result<String, ()> {
        serde_json::to_string(self).map_err(|_| ())
    }

    /// Creates a new WrappedResponse from a Command and a Uuid
    pub fn wrap(&self, uuid: Uuid) -> WrappedResponse {
        WrappedResponse {
            uuid: uuid,
            res: self.clone(),
        }
    }
}

impl FromStr for Response {
    type Err = ();

    fn from_str(raw: &str) -> Result<Response, ()> {
        serde_json::from_str(raw).map_err(|_| ())
    }
}

/// A Response bound to a UUID
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct WrappedResponse {
    pub uuid: Uuid,
    pub res: Response,
}

impl WrappedResponse {
    pub fn to_string(&self) -> Result<String, ()> {
        serde_json::to_string(self).map_err(|_| ())
    }

    /// Creates a new WrappedResponse from a Response and a Uuid
    pub fn from_response(res: Response, uuid: Uuid) -> WrappedResponse {
        WrappedResponse {
            uuid: uuid,
            res: res,
        }
    }
}

impl FromStr for WrappedResponse {
    type Err = ();

    fn from_str(raw: &str) -> Result<WrappedResponse, ()> {
        serde_json::from_str(raw).map_err(|_| ())
    }
}

/// Utility function to asynchronously sends off a command
pub fn send_command(cmd: &WrappedCommand, client: &redis::Client, commands_channel: &str) -> Result<(), serde_json::Error> {
    let command_string = try!(serde_json::to_string(cmd));
    redis::cmd("PUBLISH")
        .arg(commands_channel)
        .arg(command_string)
        .execute(client);
    Ok(())
}

/// Utility function to asynchronously send off a response
pub fn send_response(res: &WrappedResponse, client: &redis::Client, channel: &str) -> Result<(), serde_json::Error> {
    let ser = try!(serde_json::to_string(res));
    let res_str = &ser;
    redis::cmd("PUBLISH")
        .arg(channel)
        .arg(res_str)
        .execute(client);
    Ok(())
}

/// Parses a String into a `WrappedWResponse`
///
/// Left in for backwards compatability; use `WrappedResponse::from_str()` instead.
pub fn parse_wrapped_response(raw_res: String) -> WrappedResponse {
    serde_json::from_str::<WrappedResponse>(&raw_res)
        .expect("Unable to parse WrappedResponse from String")
}

#[test]
fn command_serialization() {
    let cmd_str = "{\"Register\":{\"channel\":\"channel\"}}";
    let cmd: Command = serde_json::from_str(cmd_str).unwrap();
    assert_eq!(cmd, Command::Register { channel: String::from("channel") });
}

#[test]
fn command_deserialization() {
    let cmd = Command::Register { channel: String::from("channel") };
    let cmd_string = serde_json::to_string(&cmd).unwrap();
    assert_eq!("{\"Register\":{\"channel\":\"channel\"}}", &cmd_string);
}

#[test]
fn response_serialization() {
    let res_str = "\"Ok\"";
    let res: Response = serde_json::from_str(res_str).unwrap();
    assert_eq!(res, Response::Ok);
}

#[test]
fn response_deserialization() {
    let res = Response::Ok;
    let res_string = serde_json::to_string(&res).unwrap();
    assert_eq!("\"Ok\"", &res_string);
}

#[bench]
fn wrappedcmd_to_string(b: &mut test::Bencher) {
    let cmd = Command::Ping;
    let wr_cmd = WrappedCommand {
        uuid: Uuid::new_v4(),
        cmd: cmd,
    };
    b.iter(|| {
        let wr_cmd = &wr_cmd;
        let _ = serde_json::to_string(wr_cmd);
    })
}

#[bench]
fn string_to_wrappedcmd(b: &mut test::Bencher) {
    let raw = "{\"uuid\":\"2f663301-5b73-4fa0-b201-09ab196ec5fd\",\"cmd\":{\"Register\":\
               {\"channel\":\"channel\"}}}";
    b.iter(|| {
        let raw = &raw;
        let _: WrappedCommand = serde_json::from_str(raw).unwrap();
    })
}

#[bench]
fn uuid_generation(b: &mut test::Bencher) {
    b.iter(|| {
        Uuid::new_v4();
    })
}
