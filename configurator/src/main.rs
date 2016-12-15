//! Configurator for the platform.  Initializes config files for the modules and sets up
//! the initial environment for platform runtime.
//!
//! This requires that the package libncurses-dev is installed!

#![feature(plugin)]
#![plugin(indoc)]

extern crate cursive;

use std::fs::File;
use std::process::{Child, Command, Stdio};
use std::path::Path;

use cursive::Cursive;
use cursive::views::{Dialog, TextView};

mod theme;
use theme::THEME;

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

/// Displays welcome and walks the user through first time configuration of the platform.
fn first_time() {
    let mut siv = Cursive::new();
    siv.set_theme(THEME);

    siv.add_layer(Dialog::around(TextView::new(
        indoc!("Welcome to the Bot4 Algorithmic Trading Platform!

                This tool will set up the environment for the trading platform.  It will walk you through the process of \
                installing all prerequisite software and initializing all necessary configuration settings for the platform's \
                modules.") ))
        .title("Welcome")
        .button("Continue", |s| redis_config(s) )
    );

    // Start the event loop
    siv.run();
}

fn redis_config(s: &mut Cursive) {
    let mut message_text =
        String::from(
            indoc!("The first thing that needs to be configured is Redis.  Redis PubSub is used as a messaging service \
                    that allows for communication between the platform's different modules.  It is recommended that you \
                    use a local instance of Redis since it's easy to create a security vulnerability if it is not properly \
                    configured.\n\n"
            )
        );

    let installed = redis_installed();

    if installed {
        message_text +=
            indoc!("I detected that you currently have Redis installed.  To use this local Redis install for the platform, \
                    select the \"Local\" option below."
            );
    } else {
        message_text +=
            indoc!("I was unable to detect a local Redis installation.  If Redis is currently installed and you \
                    want to use a local installation, please add the `redis-server` executable to the system PATH and \
                    restart the configuration process."
            );
    }

    s.pop_layer();
    s.add_layer(Dialog::text(
        message_text)
        .title("Information")
        .button("Local", |s| redis_local(s) )
        .button("Remote", |s| redis_remote(s) )
        .button("Exit", |s| s.quit() )
    )
}

fn redis_local(s: &Cursive) {
    unimplemented!();
}

fn redis_remote(s: &Cursive) {
    unimplemented!();
}

fn postgres_config(s: &mut Cursive) {
    unimplemented!();
}

pub fn redis_installed() -> bool {
    let child = Command::new("which")
        .arg("redis-server")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Unable to spawn `which redis_server`");

    let output = child.wait_with_output()
        .expect("Unable to get output from `which redis_server`");

    output.stdout.len() > 0
}

#[test]
fn redis_installed_test() {
    redis_installed();
}
