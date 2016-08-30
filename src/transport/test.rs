use serde_json;

use transport::command_server::*;

#[test]
fn command_serialization() {
    let cmd_str = "{\"AddSMA\": {\"period\": 6.64} }";
    let cmd: Command = serde_json::from_str(cmd_str).unwrap();
    assert_eq!(cmd, Command::AddSMA{period: 6.64f64});
}

#[test]
fn command_deserialization() {
    let cmd = Command::RemoveSMA{period: 6.64f64};
    let cmd_string = serde_json::to_string(&cmd).unwrap();
    assert_eq!("{\"RemoveSMA\":{\"period\":6.64}}", cmd_string.as_str());
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
    assert_eq!("\"Ok\"", res_string.as_str());
}
