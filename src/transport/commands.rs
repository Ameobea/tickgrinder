use serde_json;
use uuid::Uuid;

/// Represents a command sent to the Tick Processor
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Command {
    Ping,
    Restart,
    Shutdown,
    AddSMA{period: f64},
    RemoveSMA{period: f64},
}

/// Represents a command bound to a unique identifier that can be
/// used to link it with a Response
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct WrappedCommand {
    pub uuid: Uuid,
    pub cmd: Command
}

/// Converts a String into a WrappedCommand
/// JSON Format: {"uuid": "xxxx-xxxx", "cmd": {"CommandName":{"arg": "val"}}}
pub fn parse_wrapped_command(cmd: String) -> WrappedCommand {
    serde_json::from_str::<WrappedCommand>(cmd.as_str())
        .expect("Unable to parse WrappedCommand from String")
}

/// Represents a response from the Tick Processor to a Command sent
/// to it at some earlier point.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Response {
    Ok,
    Error{status: String},
    Pong
}

/// A Response bound to a UUID
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct WrappedResponse {
    pub uuid: Uuid,
    pub res: Response
}

/// Parses a String into a WrappedResponse
pub fn parse_wrapped_response(raw_res: String) -> WrappedResponse {
    serde_json::from_str::<WrappedResponse>(raw_res.as_str())
        .expect("Unable to parse WrappedResponse from String")
}
