use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseScroll;
use crate::inventory::{Inventory, HOTBAR_SIZE};
use crate::ui::screens::Screen;

#[derive(Component)]
pub struct HotbarSlotUi(pub usize);

pub fn spawn_hotbar(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            ..default()
        },
        DespawnOnExit(Screen::Gameplay),
    )).with_children(|parent| {
        for i in 0..HOTBAR_SIZE {
            parent.spawn((
                HotbarSlotUi(i),
                Node {
                    width: Val::Px(48.0),
                    height: Val::Px(48.0),
                    border: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.7)),
                BorderColor::from(Color::srgb(0.4, 0.4, 0.4)),
            ));
        }
    });
}

pub fn update_hotbar(
    inventory: Res<Inventory>,
    mut slot_query: Query<(&HotbarSlotUi, &mut BorderColor)>,
) {
    for (slot_ui, mut border) in &mut slot_query {
        *border = if slot_ui.0 == inventory.active_slot {
            BorderColor::from(Color::srgb(1.0, 1.0, 0.0))
        } else {
            BorderColor::from(Color::srgb(0.4, 0.4, 0.4))
        };
    }
}

pub fn cycle_hotbar(
    mut inventory: ResMut<Inventory>,
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
) {
    let number_keys = [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
        KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
        KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
    ];
    for (i, key) in number_keys.iter().enumerate() {
        if keys.just_pressed(*key) {
            inventory.active_slot = i;
        }
    }

    if scroll.delta.y != 0.0 {
        let delta = if scroll.delta.y > 0.0 { -1i32 } else { 1 };
        let new = (inventory.active_slot as i32 + delta)
            .rem_euclid(HOTBAR_SIZE as i32) as usize;
        inventory.active_slot = new;
    }
}
