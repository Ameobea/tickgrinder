//! Contains definitions of the settings schema that are used to generate the configuration directory
//! and to format the output files.

use misc::{SettingsPage, SettingRow, SettingType};

pub const PAGE_LIST: &'static [&'static SettingsPage] = &[
    &POSTGRES_SETTINGS,
    &REDIS_SETTINGS,
    &MM_SETTINGS,
    &FXCM_SETTINGS,
    &GENERAL_SETTINGS,
    &COMMANDSERVER_QUERYSERVER_SETTINGS,
    &RUNTIME_SETTINGS,
    &FUZZER_SETTINGS,
    &DATA_DOWNLOADER_SETTINGS,
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

pub const MM_SETTINGS: SettingsPage = SettingsPage {
    name: "Management/Monitoring Web Interface",
    rows: &[
        SettingRow {
            id: "mm_cache_size",
            name: "Message Cache Size",
            default: Some("2500"),
            setting_type: SettingType::Usize,
            comment: Some("How many of each Command, Response, and Log Line to keep in memory"),
        },
        SettingRow {
            id: "store_buffer_size",
            name: "Tantivy Document Store Buffer Size",
            default: Some("50000000"),
            setting_type: SettingType::Usize,
            comment: Some("How large (in bytes) to make the Tantivy writer buffer for the document store"),
        },
    ],
    comment: Some(&["Settings for configuring the Management/Monitoring Web Interface."]),
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
            comment: Some("This port must be open on the host in order for the client to communicate over it."),
        },
        SettingRow {
            id: "mm_port",
            name: "MM Port",
            default: Some("8002"),
            setting_type: SettingType::Usize,
            comment: Some("The port the MM web GUI will listen on.  Deprecated."),
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

pub const DATA_DOWNLOADER_SETTINGS: SettingsPage = SettingsPage {
    name: "Data Downloader Settings",
    comment: Some(&["Settings pertaining to the platform's data downloaders used to record or retreive data from external sources"]),
    rows: &[
        SettingRow {
            id: "iex_data_downloader_tops_url",
            name: "IEX TOPS API URL",
            default: Some("https://ws-api.iextrading.com/1.0/tops"),
            setting_type: SettingType::String,
            comment: Some("The API endpoint for the IEX top-of-book API.  Should be good default unless they change it.  If they do, please open an issue."),
        },
        SettingRow {
            id: "poloniex_ws_api_url",
            name: "Poloniex WS API URL",
            default: Some("wss://api.poloniex.com"),
            setting_type: SettingType::String,
            comment: Some("The API endpoint for the Poloniex websocket API.  Should be good as default; if it's changed please open an issue."),
        },
        SettingRow {
            id: "poloniex_http_api_url",
            name: "Poloniex HTTP API URL",
            default: Some("https://poloniex.com/public"),
            setting_type: SettingType::String,
            comment: Some("The API endpoint for the Poloniex HTTP API.  Should be good as default; if it's changed please open an issue."),
        },
        SettingRow {
            id: "poloniex_ws_cache_size",
            name: "Poloniex WebSocket Message Cache Size",
            default: Some("500"),
            setting_type: SettingType::Usize,
            comment: Some("How many messages to buffer before flushing into the sink.  The buffer is used to catch unordered messages.")
        },
    ],
};
