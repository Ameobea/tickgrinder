//! Configurator for the platform.  Initializes config files for the modules and sets up
//! the initial environment for platform runtime.
//!
//! This requires that the package libncurses-dev is installed!

#![feature(plugin)]
#![plugin(indoc)]

extern crate cursive;

use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::rc::Rc;

use cursive::Cursive;
use cursive::views::{Dialog, TextView, EditView, ListView, BoxView, IdView};
use cursive::view::{SizeConstraint, ViewWrapper};

mod theme;
use theme::THEME;

// type Settings = HashMap<String, String>;
// type SettingsRef = Arc<Mutex<HashMap<String, String>>>;

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
    pub fn write_json(filename: &str) {
        unimplemented!();
    }

    /// Reads the supplied JSON file and generates a Settings object from its contents.
    pub fn read_json(filename: &str) -> Settings {
        unimplemented!();
    }
}

const MIN15: SizeConstraint = SizeConstraint::AtLeast(10);
const FREE: SizeConstraint = SizeConstraint::Free;

fn main() {
    // Check if this is the first run of the configurator
    let path = Path::new(".platform_conf");
    if !path.exists() {
        // File::create(path).expect("Unable to create config placeholder file; do you have write permissions to this directory?");
        first_time();
    }

    // clear console + reset colored background before exiting
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
            .child("Redis Host", IdView::new("redis-host", BoxView::new(MIN15, FREE, EditView::new())))
            .child("Redis Port", IdView::new("redis-port", BoxView::new(MIN15, FREE, EditView::new().content("6379"))))
        ).title("Remote Redis Server Settings")
        .button("Ok", move |s| {
            settings.set("redis-host", &*get_by_id("redis-host", s).unwrap());
            settings.set("redis-port", &*get_by_id("redis-port", s).unwrap());
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
            path.  Once you've installed it, select \"Local\" again.create"
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
            .child("Postgres Host", IdView::new("postgres-host", BoxView::new(MIN15, FREE, EditView::new())))
            .child("Postgres User", IdView::new("postgres-user", BoxView::new(MIN15, FREE, EditView::new())))
            .child("Postgres Password", IdView::new("postgres-password", BoxView::new(MIN15, FREE, EditView::new().secret())))
            .child("Postgres Port", IdView::new("postgres-port", BoxView::new(MIN15, FREE, EditView::new().content("5432"))))
            .child("Postgres Database", IdView::new("postgres-db", BoxView::new(MIN15, FREE, EditView::new())))
        ).title("Remote PostgreSQL Server Settings")
            .button("Ok", move |s| save_settings(s, settings.clone()) )
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
                .child("Postgres User", IdView::new("postgres-user", BoxView::new(MIN15, FREE, EditView::new())))
                .child("Postgres Password", IdView::new("postgres-password", BoxView::new(MIN15, FREE, EditView::new().secret())))
                .child("Postgres Port", IdView::new("postgres-port", BoxView::new(MIN15, FREE, EditView::new())))
                .child("Postgres Database", IdView::new("postgres-db", BoxView::new(MIN15, FREE, EditView::new())))
            ).title("Local PostgreSQL Server Settings")
                .button("Ok", move |s| save_settings(s, settings.clone()) )
        )
    }
}

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

fn save_settings(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    for id in ["postgres-host", "postgres-user", "postgres-password", "postgres-port", "postgres-db"].iter() {
        let val = get_by_id(id, s);
        if val.is_some() && *id == "postgres-host" {
            settings.set(id, &(*val.unwrap()) );
        }
    }

    s.add_layer(Dialog::text(
        indoc!(
            "The trading platform has been successfully configured.  Run `run.sh` and visit `localhost:8002` in \
            your web browser to start using the platform."
        )
    ).button("Ok", |s| s.quit() ))
}

#[test]
fn redis_installed_test() {
    is_isntalled("redis-server");
}
