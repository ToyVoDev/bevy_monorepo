//! Combined client-server game where the host can play while serving other clients.
//!
//! This application allows a player to host their own game server while simultaneously
//! playing as a participant. Other players can join via WebTransport or WebSocket.

use {
    aeronet::transport::visualizer::SessionVisualizerPlugin,
    aeronet_replicon::{client::AeronetRepliconClientPlugin, server::AeronetRepliconServerPlugin},
    aeronet_websocket::{client::WebSocketClientPlugin, server::WebSocketServerPlugin},
    aeronet_webtransport::{client::WebTransportClientPlugin, server::WebTransportServerPlugin},
    bevy::prelude::*,
    bevy_egui::EguiPlugin,
    bevy_replicon::prelude::*,
    move_box::{
        AppMode, GameState, Player, PlayerColor, PlayerInput, PlayerPosition, apply_movement,
        client::{on_client_connected, on_client_connecting, on_client_disconnected},
        draw_boxes, handle_inputs, recv_input,
        server::{
            on_remote_client_connected, on_remote_client_disconnected, on_server_opened,
            on_session_request,
        },
        setup_level,
        ui::{ClientUi, HostUi, MenuUi, client_connection_ui, host_ui, menu_ui},
    },
};

fn main() -> AppExit {
    App::new()
        .add_plugins((
            // core
            DefaultPlugins,
            EguiPlugin {
                enable_multipass_for_primary_context: false,
            },
            // transport - both client and server plugins
            WebTransportClientPlugin,
            WebTransportServerPlugin,
            WebSocketClientPlugin,
            WebSocketServerPlugin,
            SessionVisualizerPlugin,
            // replication - both client and server plugins
            RepliconPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
            AeronetRepliconClientPlugin,
            AeronetRepliconServerPlugin,
        ))
        .init_state::<AppMode>()
        .init_state::<GameState>()
        .init_resource::<MenuUi>()
        .init_resource::<HostUi>()
        .init_resource::<ClientUi>()
        .enable_state_scoped_entities::<GameState>()
        .replicate::<Player>()
        .replicate::<PlayerPosition>()
        .replicate::<PlayerColor>()
        .add_client_event::<PlayerInput>(Channel::Unreliable)
        .add_systems(Startup, setup_level)
        .add_systems(
            Update,
            (
                menu_ui.run_if(in_state(AppMode::Menu)),
                host_ui.run_if(in_state(AppMode::Hosting)),
                client_connection_ui.run_if(in_state(AppMode::Client)),
                (draw_boxes, handle_inputs).run_if(in_state(GameState::Playing)),
            ),
        )
        .add_systems(
            FixedUpdate,
            (recv_input, apply_movement)
                .chain()
                .run_if(server_or_singleplayer),
        )
        .add_observer(on_server_opened)
        .add_observer(on_session_request)
        .add_observer(on_client_connecting)
        .add_observer(on_client_connected)
        .add_observer(on_client_disconnected)
        .add_observer(on_remote_client_connected)
        .add_observer(on_remote_client_disconnected)
        .run()
}
