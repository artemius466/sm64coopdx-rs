//! Multiplayer netcode — sm64coopdx's defining feature.
//!
//! Ported from `src/pc/network/`. The upstream project synchronizes *all*
//! entities and every level for multiple players: it sends object states over
//! UDP (or coopnet) and reconciles them on receipt. Full port is a later pass;
//! here we model the object-sync contract and a minimal deterministic
//! reconciliation.

/// The network system in use. Mirrors `NetworkSystem`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NetworkSystem {
    None = 0,
    Socket = 1,
    CoopNet = 2,
}

/// The network mode. Mirrors `NetworkState`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NetworkMode {
    None = 0,
    Client = 1,
    Server = 2,
}

/// A synced object state sent over the network.
///
/// Mirrors the upstream `SyncObject`: a snapshot of an object's position,
/// angle, model, and behavior, tagged with a timestamp for reconciliation.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncObject {
    pub id: u32,
    pub pos: [i16; 3],
    pub angle: [i16; 3],
    pub model: i16,
    pub behavior: u32,
    pub active: bool,
    pub timestamp: u32,
}

impl SyncObject {
    pub fn new(id: u32, timestamp: u32) -> Self {
        SyncObject {
            id,
            pos: [0; 3],
            angle: [0; 3],
            model: 0,
            behavior: 0,
            active: false,
            timestamp,
        }
    }
}

/// The network transport contract. Mirrors the socket/coopnet backends.
pub trait Network {
    /// Initialise the system (client or server).
    fn init(&mut self, mode: NetworkMode, system: NetworkSystem);
    /// Send a synced object to peers.
    fn send_object(&mut self, obj: &SyncObject);
    /// Receive pending synced objects. Returns the number processed.
    fn receive_objects(&mut self) -> usize;
    /// Whether the connection is currently live.
    fn is_connected(&self) -> bool;
}

/// A single-host (loopback) network for headless / test runs.
///
/// Objects "sent" are queued and returned by `receive_objects`, so a single
/// process can exercise the send/receive/reconcile pipeline.
pub struct LoopbackNetwork {
    mode: NetworkMode,
    system: NetworkSystem,
    connected: bool,
    outgoing: Vec<SyncObject>,
    incoming: Vec<SyncObject>,
    frame: u32,
}

impl LoopbackNetwork {
    pub fn new() -> Self {
        LoopbackNetwork {
            mode: NetworkMode::None,
            system: NetworkSystem::None,
            connected: false,
            outgoing: Vec::new(),
            incoming: Vec::new(),
            frame: 0,
        }
    }
}

impl Default for LoopbackNetwork {
    fn default() -> Self {
        LoopbackNetwork::new()
    }
}

impl Network for LoopbackNetwork {
    fn init(&mut self, mode: NetworkMode, system: NetworkSystem) {
        self.mode = mode;
        self.system = system;
        // A single host is always "connected" to itself.
        self.connected = mode != NetworkMode::None;
    }

    fn send_object(&mut self, obj: &SyncObject) {
        self.outgoing.push(*obj);
    }

    fn receive_objects(&mut self) -> usize {
        // Deliver last frame's outgoing objects as if they arrived.
        self.incoming = std::mem::take(&mut self.outgoing);
        self.incoming.len()
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
