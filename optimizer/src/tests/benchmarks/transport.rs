use test;

use serde_json;
use uuid::Uuid;
use algobot_util::transport::commands::*;

#[bench]
fn wrappedcmd_to_string(b: &mut test::Bencher) {
    let cmd = Command::AddSMA{period: 42.23423f64};
    let wr_cmd = WrappedCommand{uuid: Uuid::new_v4(), cmd: cmd};
    b.iter(|| {
        let wr_cmd = &wr_cmd;
        let _ = serde_json::to_string(wr_cmd);
    });
}

#[bench]
fn string_to_wrappedcmd(b: &mut test::Bencher) {
    let raw = "{\"uuid\":\"2f663301-5b73-4fa0-b201-09ab196ec5fd\",\"cmd\":{\"RemoveSMA\":{\"period\":5.2342}}}";
    b.iter(|| {
        let raw = &raw;
        let _: WrappedCommand  = serde_json::from_str(raw).unwrap();
    });
}
