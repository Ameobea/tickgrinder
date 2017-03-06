//! Maps for processing raw data received from the Poloniex API.  They accept data in JSON-encoded format and convert it to structures
//! that more appropriately match the data's structure.

use std::collections::HashMap;

use serde_json;

use transport::tickstream::generics::GenTickMap;
use transport::command_server::CommandServer;
use transport::textlog::debug;
use trading::tick::GenTick;

/// Attempts to parse the given JSON-encoded `String` into a `String`:`String` `HashMap`.  `map_name` is used for logging.
fn parse_json_hashmap(json: &str, map_name: &str, cs: &mut CommandServer) -> Result<HashMap<String, String>, ()> {
    match serde_json::from_str(json) {
        Ok(hm) => Ok(hm),
        Err(err) => {
            cs.error(Some(map_name), &format!("Error while converting `String` into `HashMap`: {:?}", err));
            return Err(());
        }
    }
}

/// Processes JSON-encoded `String`s and outputs structs containing the description of the book modification.
pub struct PoloniexBookModifyMap {
    cs: CommandServer,
}

/// Represents a modification to a Poloniex order book
#[derive(PartialEq, Debug, RustcEncodable, RustcDecodable)]
pub struct PolniexOrderBookModification {
    rate: f32,
    is_bid: bool,
    amount: f32,
}

/// Attempts to parse a `HashMap` into a `PolniexOrderBookModification`
///
/// The input JSON looks like this: {"rate": "0.00300888", "type": "bid", "amount": "3.32349029"}
fn parse_book_modification_hashmap(hm: &HashMap<String, String>) -> Result<PolniexOrderBookModification, String> {
    Ok(PolniexOrderBookModification {
        rate: hm.get("rate").unwrap().parse().map_err(debug)?,
        is_bid: hm.get("type").unwrap() == "bid",
        amount: hm.get("amount").unwrap().parse().map_err(debug)?,
    })
}

impl GenTickMap<String, PolniexOrderBookModification> for PoloniexBookModifyMap {
    fn new(_: HashMap<String, String>, cs: CommandServer) -> Self {
        PoloniexBookModifyMap {
            cs: cs,
        }
    }

    fn map(&mut self, tick: GenTick<String>) -> Option<GenTick<PolniexOrderBookModification>> {
        // parse the JSON-encoded input string into a `String`:`String` `HashMap`
        let hm = match parse_json_hashmap(&tick.data, "PoloniexBookModifyMap", &mut self.cs) {
            Ok(hm) => hm,
            Err(()) => { return None; },
        };

        // make sure that `HashMap` contains all the required fields to avoid panics
        if !hm.contains_key("rate") || !hm.contains_key("type") || !hm.contains_key("amount") {
            self.cs.error(Some("PoloniexBookModifyMap"), &format!("The parsed `HashMap` doesn't contain the required keys: {:?}", hm));
            return None;
        }

        // attempt to convert the `HashMap` into a `PoloniexOrderBookRemoval`
        match parse_book_modification_hashmap(&hm) {
            Ok(removal) => Some(GenTick {
                timestamp: tick.timestamp,
                data: removal
            }),
            Err(err) => {
                self.cs.error(Some("PoloniexBookModifyMap"), &format!("Unable to parse `HashMap` into `PoloniexOrderBookRemoval`: {:?}", err));
                None
            }
        }
    }
}

/// Processes JSON-encoded `String`s and outputs structs containing the description of the order book removal
pub struct PoloniexBookRemovalMap {
    cs: CommandServer,
}

/// Represents an order being removed from a Poloniex order book
#[derive(PartialEq, Debug, RustcEncodable, RustcDecodable)]
pub struct PoloniexOrderBookRemoval {
    rate: f32,
    is_bid: bool,
}

/// Attempts to parse a `HashMap` into a `PoloniexOrderBookRemoval`
///
/// The input JSON looks like this: {"rate": "0.00311164", type: "ask"}
fn parse_book_removal_hashmap(hm: &HashMap<String, String>) -> Result<PoloniexOrderBookRemoval, String> {
    Ok(PoloniexOrderBookRemoval {
        rate: hm.get("rate").unwrap().parse().map_err(debug)?,
        is_bid: hm.get("type").unwrap() == "bid"
    })
}

impl GenTickMap<String, PoloniexOrderBookRemoval> for PoloniexBookRemovalMap {
    fn new(_: HashMap<String, String>, cs: CommandServer) -> Self {
        PoloniexBookRemovalMap {
            cs: cs,
        }
    }

    fn map(&mut self, tick: GenTick<String>) -> Option<GenTick<PoloniexOrderBookRemoval>> {
        // parse the JSON-encoded input string into a `String`:`String` `HashMap`
        let hm = match parse_json_hashmap(&tick.data, "PoloniexBookRemovalMap", &mut self.cs) {
            Ok(hm) => hm,
            Err(()) => { return None; },
        };

        // make sure that `HashMap` contains all the required fields to avoid panics
        if !hm.contains_key("rate") || !hm.contains_key("type") {
            self.cs.error(Some("PoloniexBookRemovalMap"), &format!("The parsed `HashMap` doesn't contain the required keys: {:?}", hm));
            return None;
        }

        // attempt to convert the `HashMap` into a `PoloniexOrderBookRemoval`
        match parse_book_removal_hashmap(&hm) {
            Ok(removal) => Some(GenTick {
                timestamp: tick.timestamp,
                data: removal
            }),
            Err(err) => {
                self.cs.error(Some("PoloniexBookRemovalMap"), &format!("Unable to parse `HashMap` into `PoloniexOrderBookRemoval`: {:?}", err));
                None
            }
        }
    }
}

/// Processes JSON-encoded `String`s and outputs structs containing the description of the trade.  They expect the JSON to have this format:
///
/// {"tradeID": "364476", rate: "0.00300888", "amount": "0.03580906", "date": "2014-10-07 21:51:20", "total": "0.00010775", "type": "sell"}
pub struct PoloniexTradeMap {
    cs: CommandServer,
}

/// Represents a trade that occured on Poloniex in a particular market
#[derive(PartialEq, Debug, RustcEncodable, RustcDecodable)]
pub struct PoloniexTrade {
    trade_id: usize,
    rate: f32,
    amount: f32,
    date: String,
    total: f32,
    is_buy: bool,
}

impl GenTickMap<String, PoloniexTrade> for PoloniexTradeMap {
    fn new(_: HashMap<String, String>, cs: CommandServer) -> Self {
        PoloniexTradeMap {
            cs: cs,
        }
    }

    fn map(&mut self, tick: GenTick<String>) -> Option<GenTick<PoloniexTrade>> {
        // attempt to parse the JSON string into a `HashMap`
        let hm = match parse_json_hashmap(&tick.data, "PoloniexTradeMap", &mut self.cs) {
            Ok(hm) => hm,
            Err(()) => { return None; },
        };

        // make sure that the parsed HashMap contains all the necessary values
        if !hm.contains_key("tradeID") || !hm.contains_key("rate") || !hm.contains_key("amount") ||
                !hm.contains_key("date") || !hm.contains_key("type") {
            self.cs.error(Some("`PoloniexTradeMap`"), &format!("Required keys were not present in the parsed `HashMap`: {:?}", hm));
            return None;
        }

        // try to parse the `HashMap` into a `PoloniexTrade` and return it
        match parse_trade_hashmap(&hm) {
            Ok(trade) => Some(GenTick{
                timestamp: tick.timestamp,
                data: trade
            }),
            Err(err) => {
                self.cs.error(Some("`PoloniexTradeMap`"), &format!("Error while parsing the `HashMap` into a `PoloniexTrade`: {:?}", err));
                None
            }
        }
    }
}

/// Given a `HashMap` containing the parameters of a trade in `String`:`String`: form, attempts to parse them into a `PoloniexTrade`.
/// It is assumed that the `HashMap` is already validated to ensure that it contains the required keys; if it doesn't, this will panic.
fn parse_trade_hashmap(hm: &HashMap<String, String>) -> Result<PoloniexTrade, String> {
    // try to parse the contained values into native data types and create a `PoloniexTrade` from the raw parts
    Ok(PoloniexTrade {
        trade_id: hm.get("tradeID").unwrap().parse().map_err(debug)?,
        rate: hm.get("rate").unwrap().parse().map_err(debug)?,
        amount: hm.get("amount").unwrap().parse().map_err(debug)?,
        date: hm.get("date").unwrap().clone(),
        total: hm.get("total").unwrap().parse().map_err(debug)?,
        is_buy: hm.get("type").unwrap() == "buy",
    })
}

/// Make sure that the `PoloniexTradeMap` works as intended
#[test]
fn poloniex_trade_map() {
    use uuid::Uuid;

    let raw = String::from("{\"tradeID\": \"364476\", \"rate\": \"0.00300888\", \"amount\": \"0.03580906\", \"date\": \"2014-10-07 21:51:20\", \"total\": \"0.00010775\", \"type\": \"sell\"}");
    println!("{}", raw);
    let real = PoloniexTrade {
        trade_id: 364476,
        rate: 0.00300888f32,
        amount: 0.03580906f32,
        date: String::from("2014-10-07 21:51:20"),
        total: 0.00010775f32,
        is_buy: false,
    };

    let mut map = PoloniexTradeMap::new(HashMap::new(), CommandServer::new(Uuid::new_v4(), "`PoloniexTradeMap` Test"));
    let parsed_tick = map.map(GenTick { timestamp: 1, data: raw}).unwrap();
    assert_eq!(real, parsed_tick.data);
}

/// Make sure that the `PoloniexBookRemovalMap` works as intended
#[test]
fn poloniex_book_removal() {
    use uuid::Uuid;

    let raw = String::from("{\"rate\": \"0.00311164\", \"type\": \"ask\"}");
    let real = PoloniexOrderBookRemoval {
        rate: 0.00311164,
        is_bid: false,
    };

    let mut map = PoloniexBookRemovalMap::new(HashMap::new(), CommandServer::new(Uuid::new_v4(), "`PoloniexBookRemovalMap` Test"));
    let parsed_tick = map.map(GenTick { timestamp: 1, data: raw}).unwrap();
    assert_eq!(real, parsed_tick.data);
}

/// Make sure that the `PoloniexBookModifyMap` works as intended
#[test]
fn poloniex_book_modification() {
    use uuid::Uuid;

    let raw = String::from("{\"rate\": \"0.00300888\", \"type\": \"bid\", \"amount\": \"3.32349029\"}");
    let real = PolniexOrderBookModification {
        rate: 0.00300888f32,
        is_bid: true,
        amount: 3.32349029f32,
    };

    let mut map = PoloniexBookModifyMap::new(HashMap::new(), CommandServer::new(Uuid::new_v4(), "`PoloniexBookModifyMap` Test"));
    let parsed_tick = map.map(GenTick { timestamp: 1, data: raw}).unwrap();
    assert_eq!(real, parsed_tick.data);
}

