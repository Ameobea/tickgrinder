//! The directory displays all the available settings and lets users change them
//! from within the configurator.

use cursive::Cursive;
use cursive::views::{Dialog, TextView, EditView, ListView, BoxView, LinearLayout, SelectView};

use super::*;

/// Draws the global configuration directory which contains all possible settings and their current values.  Users can
/// page through the various configuration settings and modify them as they desire.
pub fn show_directory(s: &mut Cursive, settings: Settings, needs_start: bool) {
    let settings_ = settings.clone();
    let mut sv: SelectView<&'static SettingsPage> = SelectView::new()
        .popup()
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
        .child(sv.v_align(VAlign::Top).fixed_width(35).with_id("directory-category"))
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
            s.add_layer(get_save_dialog(last_page_ix, last_page_ix, settings__.clone(), true));
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
pub fn set_directory_comment(comment: Option<&str>, s: &mut Cursive) {
    let mut comment_box = s.find_id::<TextView>("directory-comment").unwrap();
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
pub fn get_save_dialog(last_page_ix: usize, new_page_ix: usize, settings: Settings, quit: bool) -> Dialog {
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
pub fn get_row_by_name(page: &'static SettingsPage, name: &str) -> &'static SettingRow {
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
    let mut lv: &mut ListView = &mut *s.find_id("directory-lv").expect("directory-lv not found");
    populate_list_view(new_page, lv, settings.clone());
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

/// Takes a `SettingsPage` and `ListView` and fills the `ListView` with the `SettingRow`s contained
/// inside the `SettingsPage`.
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

/// Returns true if the values any of the `EditView`s with IDs corresponding to the `SettingsRow`s
/// from the given page differ from the default values for that page.
fn check_changes(s: &mut Cursive, page: &SettingsPage, settings: Settings) -> bool {
    for row in page.iter() {
        let cur_val = get_by_id(row.id, s)
            .expect(&format!("Unable to get {} by id!", row.id));
        let last_val_opt = settings.get(String::from(row.id));
        let last_val = if last_val_opt.is_none() {
            String::from(row.default.expect(&format!("No past val for {} and no default!", row.id)))
        } else {
            last_val_opt.unwrap()
        };
        if last_val != *cur_val {
            return true
        }
    }
    false
}

/// Commits all changes for a page to the internal Settings object and then writes them to all files.
fn save_changes(s: &mut Cursive, page: &SettingsPage, settings: Settings) {
    for row in page.iter() {
        let cur_val = get_by_id(row.id, s).expect(&format!("Couldn't get value by id for: {}", row.id));
        settings.set(row.id, &*cur_val);
    }

    write_settings(settings);
}
