//! Demo app where clients can connect to a server and control a box with the
//! arrow keys.
//!
//! Box positions are synced between clients and servers using [`bevy_replicon`]
//! with the [`aeronet_replicon`] backend.
//!
//! This example currently runs the following IO layers at once:
//! - [`aeronet_websocket`] on port `25570`
//! - [`aeronet_webtransport`] on port `25571`
//!
//! Based on <https://github.com/projectharmonia/bevy_replicon_renet/blob/master/examples/simple_box.rs>.
//! Based on <https://github.com/aecsocket/aeronet/blob/main/examples/src/move_box.rs>.
//!
//! # Usage
//!
//! ## Server
//!
//! ```sh
//! cargo run --bin move_box -- --headless
//! ```
//!
//! ## Client
//!
//! Native:
//!
//! ```sh
//! cargo run --bin move_box
//! ```
//!
//! WASM:
//!
//! ```sh
//! cargo install wasm-server-runner
//! cargo run --bin move_box --target wasm32-unknown-unknown
//! ```
//!
//! You must use a Chromium browser to try the demo:
//! - Currently, the WASM client demo doesn't run on Firefox, due to an issue
//!   with how `xwt` handles getting the reader for the incoming datagram
//!   stream. This results in the backend task erroring whenever a connection
//!   starts.
//! - WebTransport is not supported on Safari.
//!
//! Eventually, when Firefox is supported but you still have problems running
//! the client under Firefox (especially LibreWolf), check:
//! - `privacy.resistFingerprinting` is disabled, or Enhanced Tracking
//!   Protection is disabled for the website (see [winit #3345])
//! - `webgl.disabled` is set to `false`, so that Bevy can use the GPU
//!
//! [winit #3345]: https://github.com/rust-windowing/winit/issues/3345
//!
//! ## Connecting
//!
//! ### WebTransport
//!
//! The server binds to `0.0.0.0` by default. To connect to the server from the
//! client, you must specify an HTTPS address. For a local server, this will be
//! `https://[::1]:PORT`.
//!
//! By default, you will not be able to connect to the server, because it uses a
//! self-signed certificate which your client (native or browser) will treat as
//! invalid. To get around this, you must manually provide SHA-256 digest of the
//! certificate's DER as a base 64 string.
//!
//! When starting the server, it outputs the *certificate hash* as a base 64
//! string (it also outputs the *SPKI fingerprint*, which is different and is
//! not necessary here). Copy this string and enter it into the "certificate
//! hash" field of the client before connecting. The client will then ignore
//! certificate validation errors for this specific certificate, and allow a
//! connection to be established.
//!
//! In the browser, egui may not let you paste in the hash. You can get around
//! this by:
//! 1. clicking into the certificate hash text box
//! 2. clicking outside of the bevy window (i.e. into the white space)
//! 3. pressing Ctrl+V
//!
//! In the native client, if you leave the certificate hash field blank, the
//! client will simply not validate certificates. **This is dangerous** and
//! should not be done in your actual app, which is why it's locked behind the
//! `dangerous-configuration` flag, but is done for convenience in this example.
//!
//! ### WebSocket
//!
//! The server binds to `0.0.0.0` without encryption. You will need to connect
//! using a URL which uses the `ws` protocol (not `wss`).
//!
//! [`aeronet_webtransport`]: https://docs.rs/aeronet_webtransport
//! [`aeronet_websocket`]: https://docs.rs/aeronet_websocket
//! [`bevy_replicon`]: https://docs.rs/bevy_replicon
//! [`aeronet_replicon`]: https://docs.rs/aeronet_replicon

pub mod client;
pub mod server;
pub mod ui;

use {
    bevy::prelude::*,
    bevy_replicon::prelude::*,
    serde::{Deserialize, Serialize},
};

/// Port to run the WebSocket server on.
pub const WEB_SOCKET_PORT: u16 = 25570;

/// Port to run the WebTransport server.
pub const WEB_TRANSPORT_PORT: u16 = 25571;

/// How many units a player may move in a single second.
const MOVE_SPEED: f32 = 250.0;

/// How many times per second we will replicate entity components.
pub const TICK_RATE: u16 = 128;

/// Application mode - whether acting as host or client
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
pub enum AppMode {
    /// Main menu - choose to host or join
    #[default]
    Menu,
    /// Hosting a server and playing as local player
    Hosting,
    /// Connected to remote server as client
    Client,
}

/// Whether the game is currently being simulated or not.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
pub enum GameState {
    /// Game is not being simulated.
    #[default]
    None,
    /// Game is being simulated.
    Playing,
}

/// Marker component for a player in the game.
#[derive(Debug, Clone, Component, Serialize, Deserialize)]
#[require(StateScoped::<GameState>(GameState::Playing))]
pub struct Player;

/// Player's box position.
#[derive(Debug, Clone, Component, Deref, DerefMut, Serialize, Deserialize)]
pub struct PlayerPosition(pub Vec2);

/// Player's box color.
#[derive(Debug, Clone, Component, Deref, DerefMut, Serialize, Deserialize)]
pub struct PlayerColor(pub Color);

/// Player's inputs that they send to control their box.
#[derive(Debug, Clone, Default, Component, Event, Serialize, Deserialize)]
pub struct PlayerInput {
    /// Lateral movement vector.
    ///
    /// The client has full control over this field, and may send an
    /// unnormalized vector! Authorities must ensure that they normalize or
    /// zero this vector before using it for movement updates.
    pub movement: Vec2,
}

pub fn recv_input(
    mut inputs: EventReader<FromClient<PlayerInput>>,
    mut players: Query<&mut PlayerInput>,
) {
    for &FromClient {
        client_entity,
        event: ref new_input,
    } in inputs.read()
    {
        let Ok(mut input) = players.get_mut(client_entity) else {
            continue;
        };
        *input = new_input.clone();
    }
}

pub fn apply_movement(time: Res<Time>, mut players: Query<(&PlayerInput, &mut PlayerPosition)>) {
    for (input, mut position) in &mut players {
        // make sure to validate inputs and normalize on the authority (server) side,
        // since we're accepting arbitrary client input
        if let Some(movement) = input.movement.try_normalize() {
            // only change `position` if we actually have a movement vector to apply
            // this saves bandwidth; we don't replicate position if we don't change it
            **position += movement * time.delta_secs() * MOVE_SPEED;
        }
    }
}

pub fn setup_level(mut commands: Commands) {
    commands.spawn(Camera2d);
}

pub fn handle_inputs(mut inputs: EventWriter<PlayerInput>, input: Res<ButtonInput<KeyCode>>) {
    let mut movement = Vec2::ZERO;
    if input.pressed(KeyCode::ArrowRight) {
        movement.x += 1.0;
    }
    if input.pressed(KeyCode::ArrowLeft) {
        movement.x -= 1.0;
    }
    if input.pressed(KeyCode::ArrowUp) {
        movement.y += 1.0;
    }
    if input.pressed(KeyCode::ArrowDown) {
        movement.y -= 1.0;
    }

    inputs.write(PlayerInput { movement });
}

pub fn draw_boxes(mut gizmos: Gizmos, players: Query<(&PlayerPosition, &PlayerColor)>) {
    for (PlayerPosition(pos), PlayerColor(color)) in &players {
        gizmos.rect_2d(*pos, Vec2::ONE * 50.0, *color);
    }
}
