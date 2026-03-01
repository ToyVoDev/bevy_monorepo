use {
    crate::{
        GameState, PlayerColor, PlayerInput, PlayerPosition, WEB_SOCKET_PORT, WEB_TRANSPORT_PORT,
        client::web_socket_client_config, ui::HostUi,
    },
    aeronet::io::{
        Session,
        connection::{Disconnected, LocalAddr},
        server::Server,
    },
    aeronet_replicon::{client::AeronetRepliconClient, server::AeronetRepliconServer},
    aeronet_websocket::{client::WebSocketClient, server::WebSocketServer},
    aeronet_webtransport::{
        cert,
        server::{SessionRequest, SessionResponse, WebTransportServer},
        wtransport,
    },
    bevy::prelude::*,
    bevy_replicon::prelude::*,
    core::time::Duration,
    std::time::SystemTime,
};

pub type WebTransportServerConfig = aeronet_webtransport::server::ServerConfig;
pub type WebSocketServerConfig = aeronet_websocket::server::ServerConfig;

pub fn web_socket_server_config(port: u16) -> WebSocketServerConfig {
    WebSocketServerConfig::builder()
        .with_bind_default(port)
        .with_no_encryption()
}

pub fn web_transport_server_config(
    identity: wtransport::Identity,
    port: u16,
) -> WebTransportServerConfig {
    WebTransportServerConfig::builder()
        .with_bind_default(port)
        .with_identity(identity)
        .keep_alive_interval(Some(Duration::from_secs(1)))
        .max_idle_timeout(Some(Duration::from_secs(5)))
        .expect("should be a valid idle timeout")
        .build()
}

pub fn start_hosting(commands: &mut Commands, host_ui: &HostUi) {
    let wt_port = host_ui.wt_port.parse().unwrap_or(WEB_TRANSPORT_PORT);
    let ws_port = host_ui.ws_port.parse().unwrap_or(WEB_SOCKET_PORT);

    // Start WebTransport server
    let identity = wtransport::Identity::self_signed(["localhost", "127.0.0.1", "::1"])
        .expect("all given SANs should be valid DNS names");
    let cert = &identity.certificate_chain().as_slice()[0];
    let cert_hash = cert::hash_to_b64(cert.hash());
    info!("Certificate hash: {cert_hash}");

    let wt_config = web_transport_server_config(identity, wt_port);
    commands
        .spawn((Name::new("WebTransport Server"), AeronetRepliconServer))
        .queue(WebTransportServer::open(wt_config));

    // Start WebSocket server
    let ws_config = web_socket_server_config(ws_port);
    commands
        .spawn((Name::new("WebSocket Server"), AeronetRepliconServer))
        .queue(WebSocketServer::open(ws_config));

    // Connect host as local player via WebSocket (simpler than WebTransport)
    let local_target = format!("ws://127.0.0.1:{ws_port}");
    let client_config = web_socket_client_config();
    commands
        .spawn((Name::new("Local Host Player"), AeronetRepliconClient))
        .queue(WebSocketClient::connect(
            client_config,
            local_target.clone(),
        ));

    info!("Starting servers on WebTransport:{wt_port} and WebSocket:{ws_port}");
    info!("Host connecting as local player to {local_target}");
}

// Server-side connection events (when remote clients connect to our server)
pub fn on_remote_client_connected(
    trigger: Trigger<OnAdd, Session>,
    names: Query<&Name>,
    mut host_ui: ResMut<HostUi>,
    mut game_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    clients: Query<&ChildOf>,
) {
    let client = trigger.target();
    let Ok(&ChildOf(_server)) = clients.get(client) else {
        return;
    };

    let name = names.get(client).map(|n| n.as_str()).unwrap_or("Unknown");
    let msg = format!("Remote client {name} connected");
    info!("{}", msg);
    host_ui.log.push(msg);

    // Start the game when first client connects (including host)
    game_state.set(GameState::Playing);

    // Generate a random-looking color for the client
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("current system time should be after unix epoch")
        .as_millis();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation is what we want"
    )]
    let color = Color::srgb_u8((time * 3) as u8, (time * 5) as u8, (time * 7) as u8);

    commands.entity(client).insert((
        crate::Player,
        PlayerPosition(Vec2::ZERO),
        PlayerColor(color),
        PlayerInput::default(),
        Replicated,
    ));
}

pub fn on_remote_client_disconnected(
    trigger: Trigger<Disconnected>,
    names: Query<&Name>,
    mut host_ui: ResMut<HostUi>,
    clients: Query<&ChildOf>,
) {
    let client = trigger.target();
    let Ok(&ChildOf(_server)) = clients.get(client) else {
        return;
    };

    let name = names.get(client).map(|n| n.as_str()).unwrap_or("Unknown");
    let msg = match &*trigger {
        Disconnected::ByUser(reason) => {
            format!("Remote client {name} disconnected by user: {reason}")
        }
        Disconnected::ByPeer(reason) => {
            format!("Remote client {name} disconnected by peer: {reason}")
        }
        Disconnected::ByError(err) => {
            format!("Remote client {name} disconnected due to error: {err:?}")
        }
    };
    info!("{}", msg);
    host_ui.log.push(msg);
}

pub fn on_server_opened(
    trigger: Trigger<OnAdd, Server>,
    servers: Query<&LocalAddr>,
    mut host_ui: ResMut<HostUi>,
) {
    let server = trigger.target();
    let local_addr = servers
        .get(server)
        .expect("opened server should have a binding socket `LocalAddr`");
    let msg = format!("Server opened on {}", **local_addr);
    info!("{}", msg);
    host_ui.log.push(msg);
}

pub fn on_session_request(mut request: Trigger<SessionRequest>) {
    let client = request.target();
    info!("{client} requesting session");
    request.respond(SessionResponse::Accepted);
}
