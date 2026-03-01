use {
    crate::{
        AppMode, GameState, PlayerColor, PlayerInput, PlayerPosition,
        ui::{ClientUi, HostUi},
    },
    aeronet::{
        io::{Session, SessionEndpoint, connection::Disconnected},
        transport::{TransportConfig, visualizer::SessionVisualizer},
    },
    aeronet_replicon::client::AeronetRepliconClient,
    aeronet_webtransport::cert,
    bevy::prelude::*,
    bevy_replicon::prelude::*,
    std::time::SystemTime,
};

pub type WebSocketClientConfig = aeronet_websocket::client::ClientConfig;
pub type WebTransportClientConfig = aeronet_webtransport::client::ClientConfig;

pub fn web_socket_client_config() -> WebSocketClientConfig {
    let config = WebSocketClientConfig::builder();
    cfg_if::cfg_if! {
        if #[cfg(not(target_family = "wasm"))] {
            config.with_no_cert_validation()
        } else {
            config
        }
    }
}

#[cfg(target_family = "wasm")]
pub fn web_transport_client_config(cert_hash: String) -> WebTransportClientConfig {
    use aeronet_webtransport::xwt_web::{CertificateHash, HashAlgorithm};

    let server_certificate_hashes = match cert::hash_from_b64(&cert_hash) {
        Ok(hash) => vec![CertificateHash {
            algorithm: HashAlgorithm::Sha256,
            value: Vec::from(hash),
        }],
        Err(err) => {
            warn!("Failed to read certificate hash from string: {err:?}");
            Vec::new()
        }
    };

    WebTransportClientConfig {
        server_certificate_hashes,
        ..Default::default()
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn web_transport_client_config(cert_hash: String) -> WebTransportClientConfig {
    use {aeronet_webtransport::wtransport::tls::Sha256Digest, core::time::Duration};

    let config = WebTransportClientConfig::builder().with_bind_default();

    let config = if cert_hash.is_empty() {
        warn!("Connecting without certificate validation");
        config.with_no_cert_validation()
    } else {
        match cert::hash_from_b64(&cert_hash) {
            Ok(hash) => config.with_server_certificate_hashes([Sha256Digest::new(hash)]),
            Err(err) => {
                warn!("Failed to read certificate hash from string: {err:?}");
                config.with_server_certificate_hashes([])
            }
        }
    };

    config
        .keep_alive_interval(Some(Duration::from_secs(1)))
        .max_idle_timeout(Some(Duration::from_secs(5)))
        .expect("should be a valid idle timeout")
        .build()
}

// Client-side connection events (when this app connects to remote server)
pub fn on_client_connecting(
    trigger: Trigger<OnAdd, SessionEndpoint>,
    names: Query<&Name>,
    mut client_ui: ResMut<ClientUi>,
    mut host_ui: ResMut<HostUi>,
    app_mode: Res<State<AppMode>>,
    mut commands: Commands,
) {
    let entity = trigger.target();
    let name = names.get(entity).map(|n| n.as_str()).unwrap_or("Unknown");
    let msg = format!("{name} connecting");
    info!("{}", msg);

    match *app_mode.get() {
        AppMode::Client => client_ui.log.push(msg),
        AppMode::Hosting => host_ui.log.push(msg),
        AppMode::Menu => {}
    }

    commands.entity(entity).insert(AeronetRepliconClient);
}

pub fn on_client_connected(
    trigger: Trigger<OnAdd, Session>,
    names: Query<&Name>,
    mut client_ui: ResMut<ClientUi>,
    mut host_ui: ResMut<HostUi>,
    app_mode: Res<State<AppMode>>,
    mut game_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    sessions: Query<(), (With<SessionEndpoint>, With<AeronetRepliconClient>)>,
) {
    let entity = trigger.target();
    let name = names.get(entity).map(|n| n.as_str()).unwrap_or("Unknown");

    // Only react if this is our client session (not a server's client)
    if sessions.get(entity).is_ok() {
        let msg = format!("{name} connected");
        info!("{}", msg);

        match app_mode.get() {
            AppMode::Client => client_ui.log.push(msg),
            AppMode::Hosting => {
                host_ui.log.push(msg);
                // When hosting, start the game immediately when host connects locally
                game_state.set(GameState::Playing);

                // Create a player entity for the host's local connection
                let time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("current system time should be after unix epoch")
                    .as_millis();
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "truncation is what we want"
                )]
                let color = Color::srgb_u8((time * 3) as u8, (time * 5) as u8, (time * 7) as u8);

                commands.entity(entity).insert((
                    crate::Player,
                    PlayerPosition(Vec2::ZERO),
                    PlayerColor(color),
                    PlayerInput::default(),
                    Replicated,
                ));
            }
            AppMode::Menu => {}
        }

        // For regular clients, set to playing
        if matches!(app_mode.get(), AppMode::Client) {
            game_state.set(GameState::Playing);
        }
        commands.entity(entity).insert((
            SessionVisualizer::default(),
            TransportConfig {
                max_memory_usage: 64 * 1024,
                send_bytes_per_sec: 4 * 1024,
                ..default()
            },
        ));
    }
}

pub fn on_client_disconnected(
    trigger: Trigger<Disconnected>,
    names: Query<&Name>,
    mut client_ui: ResMut<ClientUi>,
    mut host_ui: ResMut<HostUi>,
    app_mode: Res<State<AppMode>>,
    mut game_state: ResMut<NextState<GameState>>,
    sessions: Query<(), (With<SessionEndpoint>, With<AeronetRepliconClient>)>,
) {
    let session = trigger.target();

    // Only react if this is our client session (not a server's client)
    if sessions.get(session).is_ok() {
        let name = names.get(session).map(|n| n.as_str()).unwrap_or("Unknown");
        let msg = match &*trigger {
            Disconnected::ByUser(reason) => format!("{name} disconnected by user: {reason}"),
            Disconnected::ByPeer(reason) => format!("{name} disconnected by peer: {reason}"),
            Disconnected::ByError(err) => format!("{name} disconnected due to error: {err:?}"),
        };
        info!("{}", msg);

        match app_mode.get() {
            AppMode::Client => {
                client_ui.log.push(msg);
                // Only set game state to None if we're a pure client and we disconnect
                game_state.set(GameState::None);
            }
            AppMode::Hosting => {
                host_ui.log.push(msg);
                // When hosting, only set game state to None if the HOST disconnects
                // (which shouldn't happen normally, but handle it just in case)
                // Remote client disconnections shouldn't affect the host's game state
            }
            AppMode::Menu => {}
        }
    }
}
