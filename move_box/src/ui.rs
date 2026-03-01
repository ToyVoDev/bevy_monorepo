use {
    crate::{
        AppMode, GameState, WEB_SOCKET_PORT, WEB_TRANSPORT_PORT,
        client::{web_socket_client_config, web_transport_client_config},
        server::start_hosting,
    },
    aeronet::io::{Session, SessionEndpoint, connection::Disconnect, server::Server},
    aeronet_replicon::client::AeronetRepliconClient,
    aeronet_websocket::client::WebSocketClient,
    aeronet_webtransport::client::WebTransportClient,
    bevy::{ecs::query::QuerySingleError, prelude::*},
    bevy_egui::{EguiContexts, egui},
    bevy_replicon::prelude::*,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    #[default]
    WebTransport,
    WebSocket,
}

#[derive(Debug, Default, Resource)]
pub struct ClientUi {
    pub target: String,
    pub cert_hash: String,
    pub connection_type: ConnectionType,
    pub log: Vec<String>,
}

#[derive(Debug, Default, Resource)]
pub struct HostUi {
    pub wt_port: String,
    pub ws_port: String,
    pub log: Vec<String>,
}

#[derive(Debug, Default, Resource)]
pub struct MenuUi;

pub fn client_connection_ui(
    mut commands: Commands,
    mut egui: EguiContexts,
    mut app_mode: ResMut<NextState<AppMode>>,
    mut client_ui: ResMut<ClientUi>,
    sessions: Query<(Entity, &Name, Option<&Session>), With<SessionEndpoint>>,
    replicon_client: Res<RepliconClient>,
) -> Result {
    let default_wt_target = format!("https://127.0.0.1:{WEB_TRANSPORT_PORT}");
    let default_ws_target = format!("ws://127.0.0.1:{WEB_SOCKET_PORT}");

    egui::Window::new("Join Game").show(egui.ctx_mut(), |ui| {
        let stats = replicon_client.stats();
        ui.horizontal(|ui| {
            ui.label(match replicon_client.status() {
                RepliconClientStatus::Disconnected => "Disconnected",
                RepliconClientStatus::Connecting => "Connecting",
                RepliconClientStatus::Connected => "Connected",
            });
            if replicon_client.status() == RepliconClientStatus::Connected {
                ui.separator();
                ui.label(format!("RTT {:.0}ms", stats.rtt * 1000.0));
                ui.separator();
                ui.label(format!("Loss {:.1}%", stats.packet_loss * 100.0));
            }
        });

        ui.horizontal(|ui| {
            ui.radio_value(
                &mut client_ui.connection_type,
                ConnectionType::WebTransport,
                "WebTransport",
            );
            ui.radio_value(
                &mut client_ui.connection_type,
                ConnectionType::WebSocket,
                "WebSocket",
            );
        });

        let (default_target, show_cert_hash) = match client_ui.connection_type {
            ConnectionType::WebTransport => (default_wt_target, true),
            ConnectionType::WebSocket => (default_ws_target, false),
        };

        let has_session = sessions.iter().next().is_some();
        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let mut connect = false;

        ui.horizontal(|ui| {
            let connect_resp = ui.add_enabled(
                !has_session,
                egui::TextEdit::singleline(&mut client_ui.target)
                    .hint_text(format!("{default_target} | [enter] to connect")),
            );
            connect |= connect_resp.lost_focus() && enter_pressed;
            connect |= ui
                .add_enabled(!has_session, egui::Button::new("Connect"))
                .clicked();
        });

        if show_cert_hash {
            let cert_hash_resp = ui.add_enabled(
                !has_session,
                egui::TextEdit::singleline(&mut client_ui.cert_hash)
                    .hint_text("(optional) certificate hash"),
            );
            connect |= cert_hash_resp.lost_focus() && enter_pressed;
        }

        if connect && !has_session {
            let target = if client_ui.target.is_empty() {
                default_target
            } else {
                client_ui.target.clone()
            };

            match client_ui.connection_type {
                ConnectionType::WebTransport => {
                    let config = web_transport_client_config(client_ui.cert_hash.clone());
                    commands
                        .spawn(Name::new(format!("WT Client -> {target}")))
                        .queue(WebTransportClient::connect(config, target));
                }
                ConnectionType::WebSocket => {
                    let config = web_socket_client_config();
                    commands
                        .spawn(Name::new(format!("WS Client -> {target}")))
                        .queue(WebSocketClient::connect(config, target));
                }
            }
        }

        match sessions.single() {
            Ok((session, name, connected)) => {
                if connected.is_some() {
                    ui.label(format!("{name} connected"));
                    if ui.button("Disconnect").clicked() {
                        commands
                            .trigger_targets(Disconnect::new("pressed disconnect button"), session);
                    }
                } else {
                    ui.label(format!("{name} connecting"));
                    if ui.button("Cancel").clicked() {
                        commands.entity(session).despawn();
                    }
                }
            }
            Err(QuerySingleError::NoEntities(_)) => {}
            Err(QuerySingleError::MultipleEntities(_)) => {
                ui.label("Multiple sessions active");
            }
        }

        if ui.button("Back to Menu").clicked() {
            // Clean up any existing connections
            for (session, _, _) in &sessions {
                commands.entity(session).despawn();
            }
            app_mode.set(AppMode::Menu);
        }
    });
    Ok(())
}

pub fn menu_ui(
    mut commands: Commands,
    mut egui: EguiContexts,
    mut app_mode: ResMut<NextState<AppMode>>,
    mut host_ui: ResMut<HostUi>,
) -> Result {
    egui::CentralPanel::default().show(egui.ctx_mut(), |ui| {
        ui.heading("Move Box Game");
        ui.separator();

        ui.vertical_centered(|ui| {
            if ui.button("Host Game").clicked() {
                host_ui.wt_port = WEB_TRANSPORT_PORT.to_string();
                host_ui.ws_port = WEB_SOCKET_PORT.to_string();
                start_hosting(&mut commands, &host_ui);
                app_mode.set(AppMode::Hosting);
            }

            if ui.button("Join Game").clicked() {
                app_mode.set(AppMode::Client);
            }
        });
    });
    Ok(())
}

pub fn host_ui(
    mut commands: Commands,
    mut egui: EguiContexts,
    mut app_mode: ResMut<NextState<AppMode>>,
    host_ui: ResMut<HostUi>,
    servers: Query<Entity, With<Server>>,
    remote_clients: Query<&ChildOf, (With<Session>, Without<SessionEndpoint>)>,
    host_sessions: Query<Entity, (With<SessionEndpoint>, With<AeronetRepliconClient>)>,
    game_state: Res<State<GameState>>,
) -> Result {
    egui::Window::new("Host Controls").show(egui.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Status:");
            match *game_state.get() {
                GameState::None => ui.label("Waiting for connection..."),
                GameState::Playing => ui.label("Playing"),
            };
        });

        ui.horizontal(|ui| {
            ui.label("Servers:");
            ui.label(format!("{}", servers.iter().count()));
            ui.separator();
            ui.label("Remote clients:");
            ui.label(format!("{}", remote_clients.iter().count()));
        });

        if ui.button("Stop Hosting").clicked() {
            // Clean up servers and clients
            for server in &servers {
                commands.entity(server).despawn();
            }
            // Clean up host's own client session
            for session in &host_sessions {
                commands.entity(session).despawn();
            }
            app_mode.set(AppMode::Menu);
        }

        ui.separator();
        ui.label("Connection Log:");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for msg in &host_ui.log {
                    ui.label(msg);
                }
            });
    });
    Ok(())
}
