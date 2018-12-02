use std::path::PathBuf;

use cursive::Cursive;
use cursive::event::Key;
use cursive::traits::{Boxable, Identifiable, Scrollable};
use cursive::views::{Checkbox, DummyView, ListView};

pub fn show_list<'a>(input: &'a Vec<(&'a &'a PathBuf, String)>) -> Vec<&'a (&'a &'a PathBuf, String)> {
    let mut siv = Cursive::default();

    siv.set_theme(cursive::theme::Theme {
        shadow: false,
        borders: cursive::theme::BorderStyle::None,
        palette: {
            let mut palette = cursive::theme::Palette::default();
            palette.set_color("background", cursive::theme::Color::TerminalDefault);
            palette.set_color("view", cursive::theme::Color::TerminalDefault);
            palette.set_color("primary", cursive::theme::Color::TerminalDefault);
            palette.set_color("highlight", cursive::theme::Color::Dark(cursive::theme::BaseColor::Blue));
            palette
        }
    });

    let list_view = {
        let mut list = ListView::new();

        list.add_child("Please select the projects to fix. Confirm selection with <F5>.", DummyView);
        list.add_delimiter();

        for (_, label) in input.iter() {
            list.add_child(&label, Checkbox::new().with_id(label.clone()));
        }

        list.with_id("list").scrollable().full_screen()
    };

    siv.add_fullscreen_layer(list_view);

    siv.add_global_callback(Key::F5, Cursive::quit);

    siv.run();

    input.iter()
        .filter(|(_, label)| siv.find_id::<Checkbox>(&label).unwrap().is_checked())
        .collect()
}
