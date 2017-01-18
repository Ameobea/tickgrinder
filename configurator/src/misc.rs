//! Utilities for generating the output configuration files.

use std::slice::Iter;
use std::ops::Index;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::prelude::*;

use serde_json;

#[allow(dead_code)]
#[derive(PartialEq)]
pub enum SettingType {
    String,
    Usize,
    Boolean,
    OptionString,
}

#[derive(PartialEq)]
pub struct SettingRow {
    pub id: &'static str,
    pub name: &'static str,
    pub default: Option<&'static str>,
    pub setting_type: SettingType,
    pub comment: Option<&'static str>,
}

impl SettingRow {
    /// Converts the given value to be JSON-formatted or otherwise return the JSON-formatted
    /// default if the given value is None.
    pub fn json_val(&self, val: Option<String>) -> String {
        let raw_val = val.or(self.default.map(String::from)).expect(&format!("No value given for {} and no default exists.", self.id));
        match self.setting_type {
            SettingType::String | SettingType::Usize | SettingType::Boolean => format!("\"{}\"", raw_val),
            SettingType::OptionString => match raw_val.as_str() {
                "" => String::from("null"),
                _ => format!("\"{}\"", raw_val),
            },
        }
    }

    /// Same as `json_val()` except for Rust-formatted values.
    pub fn rust_val(&self, val: Option<String>) -> String {
        let raw_val = val.or(self.default.map(String::from)).expect(&format!("No value given for {} and no default exists.", self.id));
        match self.setting_type {
            SettingType::String => format!("\"{}\"", raw_val),
            SettingType::Usize => String::from(raw_val),
            SettingType::Boolean => raw_val, // Assume it's in the right format.
            SettingType::OptionString => match raw_val.as_str() {
                "" => String::from("None"),
                _ => format!("Some(\"{}\")", raw_val),
            },
        }
    }

    /// Same as `json_val()` except for JavaScript-formmated values.
    pub fn js_val(&self, val: Option<String>) -> String {
        let raw_val = val.or(self.default.map(String::from)).expect(&format!("No value given for {} and no default exists.", self.id));
        match self.setting_type {
            SettingType::String => format!("\"{}\"", raw_val),
            SettingType::Usize => String::from(raw_val),
            SettingType::Boolean => raw_val, // Assume it's in the right format.
            SettingType::OptionString => match raw_val.as_str() {
                "" => String::from("null"),
                _ => format!("\"{}\"", raw_val),
            },
        }
    }

    /// Returns the Rust type of the row; like `usize` or `&'static str`.
    pub fn rust_type(&self) -> String {
        String::from(match self.setting_type {
            SettingType::String => "&'static str",
            SettingType::Usize => "usize",
            SettingType::Boolean => "bool",
            SettingType::OptionString => "Option<&'static str>",
        })
    }
}

/// Generates the `struct Conf {...` schema for the config file as we.
pub fn gen_rust_schema() -> String {
    let mut content = String::from("pub struct Conf {\n");
    for page in PAGE_LIST {
        if page.comment.is_some(){
            content += &format!("\n    // {}\n\n", page.comment.unwrap().join("\n    "));
        }
        for row in page.iter() {
            if row.comment.is_some(){
                content += &format!("    // {}\n", row.comment.unwrap());
            }
            content += &format!("    pub {}: {},\n", row.id, row.rust_type());
        }
    }
    content += "}\n\n";

    content
}

#[derive(Clone)]
pub struct Settings {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            inner: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn set(&self, key: &str, val: &str) {
        let mut ul = self.inner.lock().unwrap();
        ul.insert(key.to_string(), val.to_string());
    }

    pub fn get(&self, key: String) -> Option<String> {
        let ul = self.inner.lock().unwrap();
        match ul.get(&key) {
            Some(val) => Some(val.clone()),
            None => None
        }
    }

    /// Dumps all settings to a JSON file that can be used to populate the Configurator from scratch
    pub fn write_json(&self, filename: &str) {
        let path = Path::new(filename);
        if !path.exists() {
            let _ = File::create(path).unwrap();
        }

        let mut file = OpenOptions::new().write(true).truncate(true).open(path).expect("Unable to open");

        let mut content = String::from("{\n");
        for page in PAGE_LIST {
            content += &json_format_page(page, self.clone());
        }
        let len = content.len();
        content.truncate(len-2); // get rid of trailing `,\n`
        content += "\n}";

        file.write_all((&(content + "\n")).as_bytes()).expect("Unable to write JSON-formatted output file.")
    }

    /// Dumps all settings into a .rs file that can be loaded by the platform's modules.
    pub fn write_rust(&self, filename: &str) {
        let path = Path::new(filename);
        if !path.exists() {
            let _ = File::create(path).unwrap();
        }

        let mut file = OpenOptions::new().write(true).truncate(true).open(path).expect("Unable to open");

        let mut content = String::from(indoc!(
            "//! TickGrinder platform configuration file.  This is AUTOMATICALY GENERATED by the configurator
            //! application, but may be manually edited.  However, manual edits will be reset whenever the
            //! configurator application is run (via first-time setup or via `make configure` in the project root).

            #![allow(dead_code)]\n\n"
        ));

        content += &gen_rust_schema();
        content += &gen_rust_struct(self.clone());

        file.write_all((&(content + "\n")).as_bytes()).expect("Unable to write Rust-formatted output file.");
    }

    /// Dumps the settings into a .js file exporting a conf module with the settings.
    pub fn write_js(&self, filename: &str) {
        let path = Path::new(filename);
        if !path.exists() {
            let _ = File::create(path).unwrap();
        }

        let mut file = OpenOptions::new().write(true).truncate(true).open(path).expect("Unable to open");

        let mut content = String::from(indoc!(
            "// TickGrinder JavaScript configuration file.  This is AUTOMATICALY GENERATED by the configurator
            // application, but may be manually edited.  However, manual edits will be reset whenever the
            // configurator application is run (via first-time setup or via `make configure` in the project root).

            var conf = {"
        ));

        for page in PAGE_LIST {
            if page.comment.is_some(){
                content += &format!("\n    // {}\n\n", page.comment.unwrap().join("\n    "));
            }
            for row in page.iter() {
                if row.comment.is_some(){
                    content += &format!("    // {}\n", row.comment.unwrap());
                }
                content += &format!("    {}: {},\n", row.id, row.js_val(
                    self.get(String::from(row.id))
                ));
            }
        }

        content += "}\n\nmodule.exports = conf;";

        file.write_all((&(content + "\n")).as_bytes()).expect("Unable to write JSON-formatted output file.")
    }

    /// Reads the supplied JSON file and generates a Settings object from its contents.
    pub fn read_json(filename: &str) -> Settings {
        let path = Path::new(filename);
        if !path.exists() {
            panic!("No filename exists at that path: {:?}", path);
        }

        let mut buffer = Vec::new();
        let mut file = OpenOptions::new().read(true).open(path).expect("Unable to open input file");
        file.read_to_end(&mut buffer).expect("Unable to read file into buffer");
        let content = String::from_utf8(buffer).expect("Unable to convert buffer to String");
        let inner = serde_json::from_str::<HashMap<String, String>>(&content)
            .expect("Unable to convert String to HashMap.");

        Settings {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

pub fn gen_rust_struct(settings: Settings) -> String {
    let mut content = String::from("pub const CONF: Conf = Conf {\n");
    for page in PAGE_LIST {
        if page.comment.is_some(){
            content += &format!("\n    // {}\n", page.comment.unwrap().join("\n    "));
        }
        content += "\n";
        for row in page.iter() {
            if row.comment.is_some(){
                content += &format!("    // {}\n", row.comment.unwrap());
            }
            content += &format!("    {}: {},\n", row.id, row.rust_val(
                settings.get(String::from(row.id)
            )));
        }
    }
    content += "};";

    content
}

/// Takes a page, pulls all the values for it out of the Settings object, and creates some
/// JSON-formatted lines representing its settings with comments as applicable.
pub fn json_format_page(page: &SettingsPage, settings: Settings) -> String {
    let mut res = String::new();

    for row in page.iter() {
        let val = &row.json_val(
            settings.get(String::from(row.id))
        );
        res += &format!("    \"{}\": {},\n", row.id, val);
    }

    res
}

pub struct SettingsPage {
    pub name: &'static str,
    pub rows: &'static [SettingRow],
    pub comment: Option<&'static [&'static str]>,
}

impl Index<usize> for SettingsPage {
    type Output = SettingRow;

    fn index(&self, i: usize) -> &'static SettingRow {
        &self.rows[i]
    }
}

impl SettingsPage {
    pub fn iter(&self) -> Iter<SettingRow> {
        self.rows.iter()
    }
}

pub const POSTGRES_IDS: &'static [&'static str] = &[
    "postgres_host",
    "postgres_user",
    "postgres_password",
    "postgres_port",
    "postgres_db"
];

pub const POSTGRES_SETTINGS: SettingsPage = SettingsPage {
    name: "Postgres",
    rows: &[
        SettingRow {
            id: "postgres_host",
            name: "Host",
            default: Some("localhost"),
            setting_type: SettingType::String,
            comment: None,
        },
        SettingRow {
            id: "postgres_port",
            name: "Port",
            default: Some("5432"),
            setting_type:
            SettingType::Usize,
            comment: None,
        },
        SettingRow {id: "postgres_user",
            name: "Username",
            default: None,
            setting_type:
            SettingType::String,
            comment: None,
        },
        SettingRow {
            id: "postgres_password",
            name: "Password",
            default: None,
            setting_type: SettingType::String,
            comment: None,
        },
        SettingRow {id: "postgres_db",
            name: "Database",
            default: None,
            setting_type:
            SettingType::String,
            comment: None,
        },
    ],
    comment: Some(&["PostgreSQL Settings"]),
};

pub const REDIS_SETTINGS: SettingsPage = SettingsPage {
    name: "Redis",
    rows: &[
        SettingRow {
            id: "redis_host",
            name: "Host",
            default: Some("redis://localhost:6379/"),
            setting_type: SettingType::String,
            comment: Some("In this format: redis://hostname:port/"),
        },
    ],
    comment: Some(&["Redis Settings"]),
};

pub const FXCM_SETTINGS: SettingsPage = SettingsPage {
    name: "FXCM",
    rows: &[
        SettingRow {
            id: "fxcm_username",
            name: "Username",
            default: None,
            setting_type: SettingType::String,
            comment: None,
        },
        SettingRow {
            id: "fxcm_password",
            name: "Password",
            default: None,
            setting_type: SettingType::String,
            comment: None,
        },
        SettingRow {
            id: "fxcm_url",
            name: "URL",
            default: Some("http://www.fxcorporate.com/Hosts.jsp"),
            setting_type: SettingType::String,
            comment: Some("Path to the `Hosts.jsp` file for the FXCM API."),
        },
        SettingRow {
            id: "fxcm_pin",
            name: "PIN (Optional)",
            default: Some(""),
            setting_type: SettingType::OptionString,
            comment: None,
        },
    ],
    comment: Some(&[
        "FXCM Broker Settings.  Should be valid credentials for a FXCM broker account.  You can get",
        "// a practice account that is compatible with the platform here for free with no account creation",
        "// or registration required: https://www.fxcm.com/forex-trading-demo/",
    ])
};

pub const GENERAL_SETTINGS: SettingsPage = SettingsPage {
    name: "General",
    rows: &[
        SettingRow {
            id: "redis_responses_channel",
            name: "Responses Channel",
            default: Some("responses"),
            setting_type: SettingType::String,
            comment: Some("Changing this will currently break the platform; it's just here for backwards compatibility."),
        },
        SettingRow {
            id: "redis_control_channel",
            name: "Control Channel",
            default: Some("control"),
            setting_type: SettingType::String,
            comment: Some("Changing this will currently break the platform; it's just here for backwards compatibility."),
        },
        SettingRow {
            id: "redis_log_channel",
            name: "Log Channel",
            default: Some("log"),
            setting_type: SettingType::String,
            comment: Some("The redis pub/sub channel on which log messages will be sent."),
        },
        SettingRow {
            id: "data_dir",
            name: "Data Directory",
            default: None,
            setting_type: SettingType::String,
            comment: Some("Data directory for the platform where things like historical ticks and settings are stored."),
        },
        SettingRow {
            id: "websocket_port",
            name: "MM Websocket Port",
            default: Some("7037"),
            setting_type: SettingType::Usize,
            comment: Some("This is currently hard-coded in the client-side JS for MM, so don't change this either."),
        },
        SettingRow {
            id: "mm_port",
            name: "MM Port",
            default: Some("8002"),
            setting_type: SettingType::Usize,
            comment: Some("The port the MM web GUI will listen on."),
        },
        SettingRow {
            id: "node_binary_path",
            name: "NodeJS Binary Path",
            default: None,
            setting_type: SettingType::String,
            comment: Some("The absolute path to the `node` binary."),
        },
        SettingRow {
            id: "redis_server_binary_path",
            name: "Redis Server Path",
            default: Some(""),
            setting_type: SettingType::OptionString,
            comment: Some("The absolute path to the `redis-server` executable.  Empty if Redis is installed remotely."),
        },
        SettingRow {
            id: "logger_persistance_table",
            name: "Logger Table Name",
            default: Some("logs"),
            setting_type: SettingType::String,
            comment: None,
        },
    ],
    comment: None,
};

pub const COMMANDSERVER_QUERYSERVER_SETTINGS: SettingsPage = SettingsPage {
    name: "CommandServer + QueryServer Settings",
    rows: &[
        SettingRow {
            id: "cs_timeout",
            name: "CommandServer Timeout",
            default: Some("399"),
            setting_type: SettingType::Usize,
            comment: Some(indoc!(
                "The timeout of commands sent in ms.  If a response isn't recieved within the timeout window, \
                the command is re-sent."
            )),
        },
        SettingRow {
            id: "conn_senders",
            name: "CommandServer Worker Count",
            default: Some("4"),
            setting_type: SettingType::Usize,
            comment: None,
        },
        SettingRow {
            id: "cs_max_retries",
            name: "Max CommandServer message retransmit attempts",
            default: Some("3"),
            setting_type: SettingType::Usize,
            comment: None,
        },
        SettingRow {
            id: "qs_connections",
            name: "QueryServer Worker Count",
            default: Some("4"),
            setting_type: SettingType::Usize,
            comment: None,
        },
        SettingRow {
            id: "database_conns",
            name: "QueryServer DB Connection Count",
            default: Some("10"),
            setting_type: SettingType::Usize,
            comment: None,
        },
    ],
    comment: Some(&["CommandServer/QueryServer settings.  You can leave these at defaults safely."]),
};

pub const RUNTIME_SETTINGS: SettingsPage = SettingsPage {
    name: "Runtime Settings",
    rows: &[
        SettingRow {
            id: "kill_stragglers",
            name: "Kill Stragglers",
            default: Some("true"),
            setting_type: SettingType::Boolean,
            comment: Some("If instances from a previous spawner are detected when the spawner spawns, kill them?"),
        },
        SettingRow {
            id: "reset_db_on_load",
            name: "Reset DB On Load",
            default: Some("false"),
            setting_type: SettingType::Boolean,
            comment: Some("If true, entire PostgreSQL database will be wiped every time a Tick Processor is spawned."),
        },
    ],
    comment: None,
};

pub const FUZZER_SETTINGS: SettingsPage = SettingsPage {
    name: "Fuzzer Settings",
    comment: Some(&[
        "Settings for configuring the fuzzer strategy used to test broker shims.",
        "// For more info, see README.md in /private/strategies/fuzzer",
    ]),
    rows: &[
        SettingRow {
            id: "fuzzer_deterministic_rng",
            name: "Use Deterministic RNG",
            default: Some("true"),
            setting_type: SettingType::Boolean,
            comment: Some("Set if the RNG used to generate the actions of the fuzzer should be seeded with the same seed every run."),
        },
        SettingRow {
            id: "fuzzer_seed",
            name: "Seed String",
            default: Some("S0 R4nD0m"),
            setting_type: SettingType::String,
            comment: Some("The string from which the fuzzer's RNG is seeded from (if the option is enabled)."),
        },
    ],
};

pub const PAGE_LIST: &'static [&'static SettingsPage] = &[
    &POSTGRES_SETTINGS,
    &REDIS_SETTINGS,
    &FXCM_SETTINGS,
    &GENERAL_SETTINGS,
    &COMMANDSERVER_QUERYSERVER_SETTINGS,
    &RUNTIME_SETTINGS,
    &FUZZER_SETTINGS,
];
