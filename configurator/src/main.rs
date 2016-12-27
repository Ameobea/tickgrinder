//! Configurator for the platform.  Initializes config files for the modules and sets up
//! the initial environment for platform runtime.
//!
//! This requires that the package libncurses-dev is installed!

#![feature(plugin)]
#![plugin(indoc)]

extern crate cursive;
extern crate serde_json;

use std::process::{Command, Stdio};
use std::path::Path;
use std::rc::Rc;
use std::panic::{set_hook, take_hook};

use cursive::Cursive;
#[allow(unused_imports)]
use cursive::views::{Dialog, TextView, EditView, ListView, BoxView, LinearLayout, SelectView, Panel};
use cursive::view::SizeConstraint;
use cursive::direction::Orientation;
use cursive::traits::*;

mod theme;
use theme::THEME;
mod misc;
use misc::*;

const MIN15: SizeConstraint = SizeConstraint::AtLeast(10);
const FREE: SizeConstraint = SizeConstraint::Free;

fn main() {
    // Register panic hook to reset terminal settings on panic so we can see the error
    let old_hook = take_hook();
    set_hook(Box::new(move |p| {
        clear_console();
        old_hook(p);
    }));

    // Check if this is the first run of the configurator
    let path = Path::new("settings.json");
    if !path.exists() {
        first_time();

        // clear console + reset colored background before exiting
        clear_console();
    } else {
        // if we're already configured, just read in the old settings and re-generate Rust config files.
        let settings = Settings::read_json("settings.json");
        write_settings(&mut Cursive::new(), settings);
        clear_console();
        println!("{}", &format!("{}[35mSettings files have been regenerated.", 27 as char));
        println!("{}", &format!("However, the platform must be rebuilt (`make`) in order for any changes to be reflected.{}[0m\n", 27 as char));
        println!("Edit `settings.json` in the `configurator` directory and run `make config` again to change settings.");
        println!("Delete `settings.json` and re-run configurator to start from scratch.");
    }
}

/// Clears all custom colors and formatting, restoring the terminal to defaults and clearing it.
fn clear_console() {
    print!("{}[0m{}[2J", 27 as char, 27 as char);
}

/// Returns the content of the EditView with the given ID.
fn get_by_id(id: &str, s: &mut Cursive) -> Option<Rc<String>> {
    match s.find_id::<EditView>(id) {
        Some(container) => Some(container.get_content()),
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
            .button("Continue", move |s| {
                if !is_installed("node") {
                    s.add_layer(Dialog::text(indoc!(
                        "NodeJS is required in order to run the platform's Management+Monitoring (MM) Web GUI.

                        Please install NodeJS and add the `node` binary to the system path before installing the platform."
                    )).button("Ok", |s| s.quit() ));
                }
                settings.set("node_binary_path", &which("node"));
                redis_config(s, settings.clone());
            })
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
                settings.set("redis_host", &format!(
                    "redis://localhost:{}/",
                    port
                ));
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
            settings.set("redis_host", &format!(
                "redis://{}:{}/",
                &*get_by_id("redis_host", s).unwrap(),
                &*get_by_id("redis_port", s).unwrap()
            ));
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
        match check_data_dir(&*(dir.clone()).unwrap()) {
            Ok(()) => {
                settings.set("data_dir", &*dir.unwrap());
                show_directory(s, settings.clone())
            },
            Err(err) => {
                error_popup(s, err)
            },
        };
    });
    s.add_layer(dialog)
}

/// Draws the global configuration directory which contains all possible settings and their current values.  Users can
/// page through the various configuration settings and modify them as they desire.
/// -------
/// This is currently broken; probably an internal Cursive bug.  Will work on implementing this later.  For now, it just
/// calls `write_settings` and calls it a day.
fn show_directory(s: &mut Cursive, settings: Settings) {
    write_settings(s, settings);
    // let settings_ = settings.clone();
    // s.pop_layer();
    // let mut sv: SelectView<SettingsPage> = SelectView::new()
    //     .popup()
    //     .on_submit(move |s, new_page| {
    //         let last_page_name = settings_.get(String::from("last-page"))
    //             .expect("`last-page` wasn't in settings.");
    //         let last_page = PAGE_MAPPINGS.get(last_page_name.as_str()).expect("Last page name wasn't in mapping");
    //         let changed = check_changes(s, *last_page, settings_.clone());
    //         if changed {
    //             // s.add_layer(get_save_dialog(last_page, *new_page, settings_.clone()));
    //         } else {
    //             switch_categories(s, new_page, settings_.clone());
    //         }
    //     });
    // sv.add_item("postgres", POSTGRES_SETTINGS);
    // settings.set("last-page", "postgres");
    // for (k, v) in PAGE_MAPPINGS.iter() {
    //     if *k != "postgres" {
    //         sv.add_item(*k, *v);
    //     }
    // }
    // let directory = Dialog::around(LinearLayout::new(Orientation::Vertical)
    //     .child(sv.v_align(VAlign::Center).fixed_size((20, 10)).with_id("directory-category"))
    //     .child(ListView::new().with_id("directory-lv").fixed_width(30))
    // ).button("Exit", |s| s.quit() );
    // s.add_layer(directory);
    // // s.add_layer(Dialog::around(sv.fixed_size))
    // switch_categories(s, &POSTGRES_SETTINGS, settings);
}

/// Returns the Dialog shown when switching between different settings categories in the main settings catalog.
/// If Save is selected, the changes are written written immediately to the Settings object as well as
/// copied to all applicable files.  Also handles setting the new page up.
fn get_save_dialog(last_page: SettingsPage, new_page: SettingsPage, settings: Settings) -> Dialog {
    let settings_ = settings.clone();
    Dialog::text("You have unsaved changes!  Do you want to preserve them?")
        .button("Save", move |s| {
            save_changes(s, &last_page, settings.clone());
        }).button("Discard", move |s| {
            switch_categories(s, &new_page, settings_.clone());
        })
}

fn switch_categories(s: &mut Cursive, new_page: &SettingsPage, settings: Settings) {
    let lv: &mut ListView = s.find_id("directory-lv").expect("directory-lv not found");
    populate_list_view(new_page, lv);
    // do a dirty manual reverse lookup
    let mut cur_page_name_opt = None;
    for page in PAGE_LIST.iter() {
        if page.name == new_page.name {
            cur_page_name_opt = Some(page.name);
        }
    }
    settings.set("last-page", cur_page_name_opt.expect("Failed to reverse lookup last page name."));
}

/// Takes a SettingsPage and ListView and fills the ListView with the SettingRows contained inside the SettingsPage.
fn populate_list_view(page: &SettingsPage, lv: &mut ListView) {
    lv.clear();
    for row in page.iter() {
        let mut ev = EditView::new();
        if row.default.is_some() {
            ev.set_content(row.default.unwrap());
        }
        lv.add_child(row.name, BoxView::new(MIN15, FREE, ev.with_id(row.id)))
    }
}

/// Returns true if the values any of the EditViews with IDs corresponding to the SettingsRows from the given page
/// differ from the default values for that page.
fn check_changes(s: &mut Cursive, page: SettingsPage, settings: Settings) -> bool {
    for row in page.iter() {
        let cur_val = get_by_id(row.id, s).unwrap();
        let last_val = settings.get(String::from(row.id))
            .expect(&format!("Unable to get past val in check_changes: {}", row.id));
        if last_val != *cur_val {
            return true
        }
    }
    false
}

/// Commits all changes for a page to the internal Settings object and then writes them to all files.
fn save_changes(s: &mut Cursive, page: &SettingsPage, settings: Settings) {
    for row in page.iter() {
        let cur_val = get_by_id(row.id, s).unwrap();
        settings.set(row.id, &*cur_val);
    }

    write_settings(s, settings);
}

/// Runs `which [command]` and returns true if the binary is located.
fn is_installed(binary: &str) -> bool {
    let res = which(binary);

    res.len() > 0
}

fn which(binary: &str) -> String {
    let child = Command::new("which")
        .arg(binary)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Unable to spawn `which redis_server`");

    let output = child.wait_with_output()
        .expect("Unable to get output from `which redis_server`");

    String::from(String::from_utf8(output.stdout).expect("Couldn't convert UTF8 buffer to String").trim())
}

/// Creates an error popup with the supplied message and a button to dismiss it.
fn error_popup(s: &mut Cursive, err_str: &str) {
    s.add_layer(Dialog::text(err_str).dismiss_button("Ok"));
}

/// Display completion message and write the entered settings to a JSON file.
fn write_settings(s: &mut Cursive, settings: Settings) {
    settings.write_json("settings.json");
    settings.write_rust("conf.rs");
    settings.write_js("conf.js");

    s.pop_layer();

    s.add_layer(Dialog::text(
        indoc!(
            "The trading platform has been successfully configured.  Run `run.sh` and visit `localhost:8002` in \
            your web browser to start using the platform.

            To change settings, edit `config.json` and re-run the configurator via running `make configure` in the \
            project root.  To start the configuration process over and reset all settings, delete `config.json` and \
            re-run the configurator."
        )
    ).button("Ok", move |s| {
        s.quit()
    }))
}

/// Attempts to read the values of all fields with the supplied IDs from the Cursive object and write them
/// into the Settings object.  Ignores them if such an ID doesn't exist.
fn save_settings(s: &mut Cursive, settings: Settings, ids: &[&str]) {
    for id in ids {
        let val = get_by_id(id, s);
        if val.is_some() {
            settings.set(id, &(*val.unwrap()) );
        }
    }
}

/// Returns Ok if the user's selected data directory is good to use and an Err with a reason why not otherwise.
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

#[test]
fn redis_installed_test() {
    is_installed("redis-server");
}
