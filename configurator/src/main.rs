//! Configurator for the platform.  Initializes config files for the modules and sets up
//! the initial environment for platform runtime.
//!
//! This requires that the package libncurses-dev is installed!

#![feature(plugin)]
#![plugin(indoc)]

extern crate cursive;
extern crate serde_json;

use std::fs::{File, OpenOptions};
use std::process::{Command, Stdio};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::io::prelude::*;
use std::panic::set_hook;

use cursive::Cursive;
use cursive::views::{Dialog, TextView, EditView, ListView, BoxView, LinearLayout};
use cursive::view::{SizeConstraint, ViewWrapper};
use cursive::direction::Orientation;
use cursive::traits::*;

mod theme;
use theme::THEME;

struct SettingRow {
    pub id: &'static str,
    pub name: &'static str,
    pub default: Option<&'static str>,
}

type SettingsPage = &'static [SettingRow];

const POSTGRES_IDS: &'static [&'static str] = &["postgres_host", "postgres_user", "postgres_password", "postgres_port", "postgres_db"];

const FXCM_SETTINGS: SettingsPage = &[
    SettingRow {id: "fxcm_username", name: "Username", default: Some("D102691234567") },
    SettingRow {id: "fxcm_password", name: "Password", default: Some("1234")},
    SettingRow {id: "fxcm_url", name: "URL", default: Some("http://www.fxcorporate.com/Hosts.jsp") },
    SettingRow {id: "fxcm_pin", name: "PIN (Can leave blank)", default: None },
];

#[derive(Clone)]
struct Settings {
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

    /// Dumps the Settings object to a JSON file that can be used to populate the Settings object from scratch
    pub fn write_json(&self, filename: &str) {
        let path = Path::new(filename);
        if !path.exists() {
            let _ = File::create(path).unwrap();
        }

        let mut file = OpenOptions::new().write(true).open(path).expect("Unable to open");
        let inner = self.inner.lock().unwrap();
        let content = serde_json::to_string_pretty(&*inner).expect("Unable to serialize settings!");
        file.write_all((&(content + "\n")).as_bytes()).expect("Unable to write into output file.")
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

const MIN15: SizeConstraint = SizeConstraint::AtLeast(10);
const FREE: SizeConstraint = SizeConstraint::Free;

fn main() {
    // Register panic hook to reset terminal settings on panic so we can see the error
    set_hook(Box::new(|_| {
        clear_console();
    }));

    // Check if this is the first run of the configurator
    let path = Path::new(".platform_conf");
    if !path.exists() {
        // File::create(path).expect("Unable to create config placeholder file; do you have write permissions to this directory?");
        first_time();
    }

    // clear console + reset colored background before exiting
    clear_console();
}

/// Clears all custom colors and formatting, restoring the terminal to defaults and clearing it.
fn clear_console() {
    print!(".{}[0m{}[2J", 27 as char, 27 as char);
}

fn get_by_id(id: &str, s: &mut Cursive) -> Option<Rc<String>> {
    match s.find_id::<BoxView<EditView>>(id) {
        Some(container) => Some(container.get_view().get_content()),
        None => None
    }
}

/// Displays welcome and walks the user through first time configuration of the platform.
fn first_time() {
    let mut siv = Cursive::new();
    siv.set_theme(THEME);

    // Main Key:Value settings for the application
    let settings = Settings::new();

    siv.add_layer(
        Dialog::around(TextView::new(
            indoc!(
                "Welcome to the Bot4 Algorithmic Trading Platform!

                This tool will set up the environment for the trading platform.  It will walk you through the process of \
                installing all prerequisite software and initializing all necessary configuration settings for the platform's \
                modules."
            )
        )).title("Welcome")
            .button("Continue", move |s| redis_config(s, settings.clone()) )
    );

    // Start the event loop
    siv.run();
}

fn redis_config(s: &mut Cursive, settings: Settings) {
    let settings_clone = settings.clone();

    let mut message_text =
        String::from(
            indoc!(
                "The first thing that needs to be configured is Redis.  Redis PubSub is used as a messaging service \
                that allows for communication between the platform's different modules.  It is recommended that you \
                use a local instance of Redis since it's easy to create a security vulnerability if it is not properly \
                configured.\n\n"
            )
        );

    let installed = is_installed("redis-server");

    if installed {
        message_text +=
            indoc!(
                "I detected that you currently have Redis installed.  To use this local Redis install for the platform, \
                select the \"Local\" option below."
            );
    } else {
        message_text +=
            indoc!(
                "I was unable to detect a local Redis installation.  If Redis is currently installed and you \
                want to use a local installation, please add the `redis-server` executable to the system PATH and \
                restart the configuration process."
            );
    }

    s.pop_layer();
    s.add_layer(Dialog::text(message_text)
        .title("Information")
        .button("Local", move |s| redis_local(s, is_installed("redis-server"), settings.clone()) )
        .button("Remote", move |s| redis_remote(s, settings_clone.clone()) )
        .button("Exit", |s| s.quit() )
    )
}

fn redis_local(s: &mut Cursive, installed: bool, settings: Settings) {
    settings.set("redis_host", "localhost");

    if !installed {
        s.add_layer(Dialog::text(
            indoc!(
                "You must install redis in order to use it locally.  Install Redis, add the `redis-server` \
                binary to to the system PATH, and select local again."
            )
        ).dismiss_button("Ok"))
    } else {
        s.pop_layer();
        let mut port_box = EditView::new()
            .on_submit(move |s, port| {
                settings.set("redis_port", port);
                postgres_config(s, settings.clone())
            });
        port_box.set_content("6379");

        s.add_layer(Dialog::around(port_box)
            .title("Redis Port")
        );
    }
}

fn redis_remote(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    s.add_layer(Dialog::new()
        .content(ListView::new()
            .child("Redis Host", BoxView::new(MIN15, FREE, EditView::new().with_id("redis_host")))
            .child("Redis Port", BoxView::new(MIN15, FREE, EditView::new().content("6379").with_id("redis_port")))
        ).title("Remote Redis Server Settings")
        .button("Ok", move |s| {
            settings.set("redis_host", &*get_by_id("redis_host", s).unwrap());
            settings.set("redis_port", &*get_by_id("redis_port", s).unwrap());
            postgres_config(s, settings.clone())
        })
    );
}

fn postgres_config(s: &mut Cursive, settings: Settings) {
    let settings_clone = settings.clone();
    let installed = is_installed("psql");

    let mut message_text =
        String::from(
            indoc!("The platform also relies on PostgreSQL to store historical Tick data, persistant platform \
                    data, and other information for the platform.\n\n"
            )
        );

    if !installed {
        message_text += indoc!(
            "I was unable do detect an active PostgreSQL installation on this host.  In order to use this \
            host for the platform, you must first install PostgreSQL and add the `psql` binary to the system \
            path.  Once you've installed it, select \"Local\" again. "
        )
    } else {
        message_text += indoc!(
            "I detected that you have PostgreSQL installed locally.  To configure the platform to use the \
            local PostgreSQL installation, select the \"Local\" option below."
        );
    }

    s.pop_layer();
    s.add_layer(Dialog::text(message_text)
        .title("PostgreSQL Configuration")
        .button("Local", move |s| postgres_local(s, is_installed("psql"), settings.clone()) )
        .button("Remote", move |s| postgres_remote(s, settings_clone.clone()) )
        .button("Exit", |s| s.quit() )
    );
}

fn postgres_remote(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    s.add_layer(Dialog::new()
        .content(ListView::new()
            .child("Postgres Host", BoxView::new(MIN15, FREE, EditView::new().with_id("postgres_host")))
            .child("Postgres User", BoxView::new(MIN15, FREE, EditView::new().with_id("postgres_user")))
            .child("Postgres Password", BoxView::new(MIN15, FREE, EditView::new().secret().with_id("postgres_password")))
            .child("Postgres Port", BoxView::new(MIN15, FREE, EditView::new().content("5432").with_id("postgres_port")))
            .child("Postgres Database", BoxView::new(MIN15, FREE, EditView::new().with_id("postgres_db")))
        ).title("Remote PostgreSQL Server Settings")
            .button("Ok", move |s| {
                save_settings(s, settings.clone(), POSTGRES_IDS);
                set_data_dir(s, settings.clone())
            })
    )
}

fn postgres_local(s: &mut Cursive, installed: bool, settings: Settings) {
    settings.set("postgres_host", "localhost");

    if !installed {
        s.add_layer(Dialog::text(
            indoc!(
                "You must install PostgreSQL in order to use it locally.  Install PostgreSQL, add the `psql` \
                binary to to the system PATH, and select local again."
            )
        ).dismiss_button("Ok"))
    } else {
        s.pop_layer();
        s.add_layer(Dialog::new()
            .content(ListView::new()
                .child("Postgres User", BoxView::new(MIN15, FREE, EditView::new().with_id("postgres_user")))
                .child("Postgres Password", BoxView::new(MIN15, FREE, EditView::new().secret().with_id("postgres_password")))
                .child("Postgres Port", BoxView::new(MIN15, FREE, EditView::new().content("5432").with_id("postgres_port")))
                .child("Postgres Database", BoxView::new(MIN15, FREE, EditView::new().with_id("postgres_db")))
            ).title("Local PostgreSQL Server Settings").button("Ok", move |s| {
                settings.set("postgres_host", "localhost");
                save_settings(s, settings.clone(), POSTGRES_IDS);
                set_data_dir(s, settings.clone())
            })
        )
    }
}

/// Ask the user for a place to store historical data and do some basic sanity checks on the supplied path.
fn set_data_dir(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    let dialog = Dialog::around(LinearLayout::new(Orientation::Vertical)
        .child(TextView::new(
            indoc!(
                "The data directory holds all persistant storage for the platform including historical price data, \
                database dumps, and platform configuration settings.  The entry below should be the absolute path of a \
                folder that exists and is writable by the user that the platform will be run as.\n\n"
            )
        ))
        .child(ListView::new()
            .child("Data Directory", BoxView::new(MIN15, FREE, EditView::new().content("/var/bot4_data/").with_id("data_directory")))
        )
    ).title("Data Directory").button("Ok", move |s| {
        let dir = get_by_id("data_directory", s);
        match check_data_dir(&*dir.unwrap()) {
            Ok(()) => write_settings(s, settings.clone()),
            Err(err) => {
                error_popup(s, err)
            },
        };
    });
    s.add_layer(dialog)
}

/// Runs `which [command]` and returns true if the binary is located.
fn is_installed(command: &str) -> bool {
    let child = Command::new("which")
        .arg(command)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Unable to spawn `which redis_server`");

    let output = child.wait_with_output()
        .expect("Unable to get output from `which redis_server`");

    output.stdout.len() > 0
}

/// Creates an error popup with the supplied message and a button to dismiss it.
fn error_popup(s: &mut Cursive, err_str: &str) {
    s.add_layer(Dialog::text(err_str).dismiss_button("Ok"));
}

/// Write the entered settings to a JSON file.
fn write_settings(s: &mut Cursive, settings: Settings) {
    let settings_ = settings.clone();
    s.pop_layer();

    s.add_layer(Dialog::text(
        indoc!(
            "The trading platform has been successfully configured.  Run `run.sh` and visit `localhost:8002` in \
            your web browser to start using the platform."
        )
    ).button("Ok", move |s| {
        settings_.write_json("settings.json");
        s.quit()
    }))
}

/// Attempts to read the values of all fields with the supplied IDs from the Cursive object and write them
/// into the Settings object.  Ignores them if such an ID doesn't exist.
fn save_settings(s: &mut Cursive, settings: Settings, ids: &[&str]) {
    for id in ids {
        let val = get_by_id(id, s);
        if val.is_some() && *id == "postgres_host" {
            settings.set(id, &(*val.unwrap()) );
        }
    }
}

fn check_data_dir(dir: &str) -> Result<(), &'static str> {
    let path = Path::new(dir);
    if !path.exists() {
        return Err(indoc!(
            "The path you specified does not exist.  Please make sure that you supplied a directory that the \
            platform's user has full read and write access to."
        ))
    }
    // TODO: Check that the directory has the correct permissions, maybe auto-create directory if it doesn't exist.

    Ok(())
}

/// Takes a SettingsPage and generates a ListView for it.
fn gen_list_view(page: SettingsPage) -> ListView {
    let mut lv = ListView::new();
    for row in page {
        let mut ev = EditView::new();
        if row.default.is_some() {
            ev.set_content(row.default.unwrap());
        }
        lv = lv.child(row.name, BoxView::new(MIN15, FREE, ev.with_id(row.id)))
    }

    lv
}

#[test]
fn redis_installed_test() {
    is_installed("redis-server");
}
