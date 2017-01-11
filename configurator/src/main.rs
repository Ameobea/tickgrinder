//! Configurator for the platform.  Initializes config files for the modules and sets up
//! the initial environment for platform runtime.
//!
//! This requires that the package libncurses-dev is installed!

#![feature(plugin)]
#![plugin(indoc)]

extern crate cursive;
extern crate serde_json;
extern crate termion;

use std::process::{Command, Stdio};
use std::path::Path;
use std::rc::Rc;

use cursive::Cursive;
#[allow(unused_imports)]
use cursive::views::{Dialog, TextView, EditView, ListView, BoxView, LinearLayout, SelectView, Panel};
use cursive::view::SizeConstraint;
use cursive::direction::Orientation;
use cursive::align::VAlign;
use cursive::traits::*;

mod theme;
use theme::THEME;
mod misc;
use misc::*;

const MIN15: SizeConstraint = SizeConstraint::AtLeast(10);
const FREE: SizeConstraint = SizeConstraint::Free;

fn main() {
    // Check if this is the first run of the configurator
    let path = Path::new("settings.json");
    let mut s = Cursive::new();

    if !path.exists() {
        first_time(&mut s);
    } else {
        let settings = Settings::read_json("settings.json");
        show_directory(&mut s, settings.clone(), true);
    }

    // clear_console();
}

/// Called after exiting the directory.
fn directory_exit(s: &mut Cursive, settings: Settings) {
    write_settings(settings);
    let content = indoc!(
        "Settings files have been regenerated.  However, the platform must be rebuilt (`make`) \
        in order for any changes to be reflected.

        Edit `settings.json` in the `configurator` directory and run `make config` again to change settings.
        Delete `settings.json` and re-run configurator to start from scratch."
    );
    s.add_layer(Dialog::text(content)
        .button("Ok", move |s| {
            s.quit();
        })
    );
}

/// Returns the content of the EditView with the given ID.
fn get_by_id(id: &str, s: &mut Cursive) -> Option<Rc<String>> {
    match s.find_id::<EditView>(id) {
        Some(container) => Some(container.get_content()),
        None => None
    }
}

/// Displays welcome and walks the user through first time configuration of the platform.
fn first_time(siv: &mut Cursive) {
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
                initial_config_done(s, settings.clone());
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
fn show_directory(s: &mut Cursive, settings: Settings, needs_start: bool) {
    let settings_ = settings.clone();
    let mut sv: SelectView<&'static SettingsPage> = SelectView::new()
        // .popup()
        .on_submit(move |s, new_page| {
            let new_page: &&'static SettingsPage = new_page;
            let last_page_ix: usize = settings_.get(String::from("last-page"))
                .expect("`last-page` wasn't in settings.")
                .parse()
                .expect("Unable to parse last-page value into usize!");
            let last_page = PAGE_LIST[last_page_ix];
            let changed = check_changes(s, last_page, settings_.clone());
            if changed {
                let ix = get_page_index(new_page.name)
                    .expect("Unable to find the index of the new page!");
                s.add_layer(get_save_dialog(last_page_ix, ix, settings_.clone(), false));
            } else {
                switch_categories(s, new_page, settings_.clone());
            }
        });
    for page in PAGE_LIST {
        sv.add_item(page.name, *page);
    }
    settings.set("last-page", "0");

    let settings_ = settings.clone();
    let mut lv = ListView::new().on_select(move |s, label| {
        // get the currently selected page and the currently selected row of that page
        let ix: usize = settings_.get(String::from("last-page"))
            .expect("Unable to get last-page")
            .parse()
            .expect("Unable to parse last-page value into usize!");
        let page = PAGE_LIST[ix];
        let row = get_row_by_name(page, label);
        // If that row has a comment, display that comment
        set_directory_comment(row.comment, s);
    });
    populate_list_view(PAGE_LIST[0], &mut lv, settings.clone());

    let width = s.screen_size().x;
    let settings__ = settings.clone();

    let directory = Dialog::around(LinearLayout::new(Orientation::Vertical)
        .child(sv.v_align(VAlign::Top).with_id("directory-category"))
        .child(TextView::new("")
            .v_align(VAlign::Center)
            .with_id("directory-comment")
            .fixed_height(4)
        )
        .child(lv.with_id("directory-lv"))
        .fixed_width(width)
    ).button("Exit", move |s| {
        let last_page_ix: usize = settings__.get(String::from("last-page"))
            .expect("`last-page` wasn't in settings.")
            .parse()
            .expect("Unable to parse last-page value into usize!");
        let last_page = PAGE_LIST[last_page_ix];
        let changed = check_changes(s, last_page, settings__.clone());
        if changed {
            s.add_layer(get_save_dialog(0, 0, settings__.clone(), true));
        } else {
            directory_exit(s, settings__.clone());
        }
    });
    s.add_layer(directory);
    if needs_start {
        s.run();
    }
    switch_categories(s, &POSTGRES_SETTINGS, settings);
}

/// Sets the value of the directory's comment box.
fn set_directory_comment(comment: Option<&str>, s: &mut Cursive) {
    let comment_box = s.find_id::<TextView>("directory-comment").unwrap();
    match comment {
        Some(comment_str) => comment_box.set_content(comment_str),
        None => comment_box.set_content(""),
    }
}

/// Returns the Dialog shown when switching between different settings categories in the main settings catalog.
/// If Save is selected, the changes are written written immediately to the Settings object as well as
/// copied to all applicable files.  Also handles setting the new page up.
///
/// If `quit` is true, the application exits instead of switching categories.
fn get_save_dialog(last_page_ix: usize, new_page_ix: usize, settings: Settings, quit: bool) -> Dialog {
    let settings_  = settings.clone();
    let settings__ = settings.clone();
    Dialog::text("You have unsaved changes!  Do you want to preserve them?")
        .button("Save", move |s| {
            save_changes(s, PAGE_LIST[last_page_ix], settings.clone());
            if !quit {
                switch_categories(s, PAGE_LIST[new_page_ix], settings_.clone());
                s.pop_layer();
            } else {
                directory_exit(s, settings_.clone());
            }
        }).button("Discard", move |s| {
            if !quit {
                switch_categories(s, PAGE_LIST[new_page_ix], settings__.clone());
                s.pop_layer();
            } else {
                directory_exit(s, settings__.clone());
            }
        })
}

/// Given a settings page and a name, returns the row that has that name.
fn get_row_by_name(page: &'static SettingsPage, name: &str) -> &'static SettingRow {
    for row in page.iter() {
        if row.name == name {
            return row
        }
    }
    panic!("No setting row with that name in the supplied page!");
}

/// Changes to a different settings page in the directory, clearing the list of the old
/// rows and adding the rows for the new page.
fn switch_categories(s: &mut Cursive, new_page: &SettingsPage, settings: Settings) {
    // blank out the comment
    set_directory_comment(None, s);
    let lv: &mut ListView = s.find_id("directory-lv").expect("directory-lv not found");
    populate_list_view(&new_page, lv, settings.clone());
    let i = get_page_index(new_page.name)
        .expect("Unable to lookup page!");
    settings.set("last-page", &i.to_string());
}

/// Returns the index of the page with the given name.
fn get_page_index(page_name: &str) -> Option<usize> {
    for (i, page) in PAGE_LIST.iter().enumerate() {
        if page.name == page_name {
            return Some(i);
        }
    }

    None
}

/// Takes a SettingsPage and ListView and fills the ListView with the SettingRows contained inside the SettingsPage.
fn populate_list_view(page: &SettingsPage, lv: &mut ListView, settings: Settings) {
    lv.clear();
    for row in page.iter() {
        let mut ev = EditView::new();
        let val = settings.get(String::from(row.id));
        if val.is_some() {
            ev.set_content(val.unwrap());
        }
        else if row.default.is_some() {
            ev.set_content(row.default.unwrap());
        }
        lv.add_child(row.name, BoxView::new(MIN15, FREE, ev.with_id(row.id)))
    }
}

/// Returns true if the values any of the EditViews with IDs corresponding to the SettingsRows from the given page
/// differ from the default values for that page.
fn check_changes(s: &mut Cursive, page: &SettingsPage, settings: Settings) -> bool {
    for row in page.iter() {
        let cur_val = get_by_id(row.id, s)
            .expect(&format!("Unable to get {} by id!", row.id));
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

    write_settings(settings);
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

/// Writes the entered settings to a JSON file.  Also generates Rust and JavaScript config files
/// that can be copied into the project src directories.
fn write_settings(settings: Settings) {
    settings.write_json("settings.json");
    settings.write_rust("conf.rs");
    settings.write_js("conf.js");
}

/// Displays a message about how to use the directory and saves all settings to file.
fn initial_config_done(s: &mut Cursive, settings: Settings) {
    s.pop_layer();

    write_settings(settings.clone());

    s.add_layer(Dialog::text(
        indoc!(
            "The trading platform has been successfully configured.  Run `run.sh` and visit `localhost:8002` in \
            your web browser to start using the platform.

            You will now be presented with the configuration directory containing all of the platform's settings.  \
            You can reach that menu at any time by running `make configure` in the project's root directory.  If you \
            want to reset all the settings and start the configuration process from scratch, just delete the \
            `settings.json` file in the `configurator` directory and run `make config` again from the project root."
        )
    ).button("Ok", move |s| {
        show_directory(s, settings.clone(), false);
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
