//! The title screen that appears after the splash screen.

use bevy::prelude::*;

use crate::ui::{menus::Menu, screens::Screen};

const TITLE_BACKGROUND_COLOR: Color = Color::srgb(0.05, 0.05, 0.08);

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Title), (set_title_bg, open_main_menu));
    app.add_systems(OnExit(Screen::Title), close_menu);
}

fn set_title_bg(mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = TITLE_BACKGROUND_COLOR;
}

fn open_main_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Main);
}

fn close_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::None);
}
