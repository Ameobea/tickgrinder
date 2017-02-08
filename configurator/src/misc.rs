//! Utilities for generating the output configuration files.

use std::slice::Iter;
use std::ops::Index;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::prelude::*;

use serde_json;

use schema::*;

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
