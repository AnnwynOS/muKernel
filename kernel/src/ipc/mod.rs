use core::fmt::Alignment::Right;
use spin::Mutex;
use crate::capabilities::{self, CapabilityId, CapabilityKind, Rights};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MessageKind {
    Request = 0,
    Reply = 1,
    Notification = 2,
    Error = 3,
}

#[derive(Clone, Copy)]
pub struct Message {
    pub kind: MessageKind,
    pub sender: u64,
    pub data: [u8; 48],
    pub data_len: u8,
}

impl Message {
    pub const fn empty(kind: MessageKind, sender: u64) -> Self {
        Self { kind, sender, data: [0; 48], data_len: 0 }
    }

    pub fn with_data(kind: MessageKind, sender: u64, src: &[u8]) -> Self {
        let mut m = Self::empty(kind, sender);
        let n = src.len().min(48);
        m.data[..n].copy_from_slice(&src[..n]);
        m.data_len = n as u8;
        m
    }
}

const QUEUE_DEPTH: usize = 16;
const MAX_ENDPOINTS: usize = 16;

struct Queue {
    id: u64,
    used: bool,
    msgs: [Message; QUEUE_DEPTH],
    head: usize,
    tail: usize,
    count: usize,
    owner: u64
}

impl Queue {
    const fn empty() -> Self {
        Self {
            id: 0,
            used: false,
            msgs: [Message::empty(MessageKind::Request, 0); QUEUE_DEPTH],
            head: 0,
            tail: 0,
            count: 0,
            owner: 0,
        }
    }

    fn push(&mut self, msg: Message) -> bool {
        if self.count >= QUEUE_DEPTH { return false; }
        self.msgs[self.tail] = msg;
        self.tail = (self.tail + 1) % QUEUE_DEPTH;
        self.count += 1;
        true
    }

    fn pop(&mut self) -> Option<Message> {
        if self.count == 0 { return None; }
        let msg = self.msgs[self.head];
        self.head = (self.head + 1) % QUEUE_DEPTH;
        self.count -= 1;
        Some(msg)
    }
}

struct Registry {
    queues: [Queue; MAX_ENDPOINTS],
    next_id: u64,
}

impl Registry {
    const fn new() -> Self {
        Self {
            queues:  [
                Queue::empty(), Queue::empty(), Queue::empty(), Queue::empty(),
                Queue::empty(), Queue::empty(), Queue::empty(), Queue::empty(),
                Queue::empty(), Queue::empty(), Queue::empty(), Queue::empty(),
                Queue::empty(), Queue::empty(), Queue::empty(), Queue::empty(),
            ],
            next_id: 1,
        }
    }

    fn create(&mut self, owner: u64) -> Option<EndpointId> {
        for q in self.queues.iter_mut() {
            if !q.used {
                q.id = self.next_id;
                q.used = true;
                q.owner = owner;
                self.next_id += 1;
                return Some(EndpointId(q.id));
            }
        }
        None
    }

    fn get_mut(&mut self, id: EndpointId) -> Option<&mut Queue> {
        self.queues.iter_mut().find(|q| q.used && q.id == id.0)
    }

    fn destroy(&mut self, id: EndpointId) {
        for q in self.queues.iter_mut() {
            if q.used && q.id == id. 0 {
                *q = Queue::empty();
                return;
            }
        }
    }
}

static REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    PermissionDenied,
    InvalidEndpoint,
    QueueFull,
    WouldBlock,
}

pub fn create_endpoint() -> Option<(EndpointId, CapabilityId, CapabilityId)> {
    create_endpoint_for(0)
}

pub fn create_endpoint_for(owner: u64) -> Option<(EndpointId, CapabilityId, CapabilityId)> {
    let id = REGISTRY.lock().create(owner)?;
    let send_cap = capabilities::create(
        CapabilityKind::IpcEndpoint { endpoint_id: id.0 }, Rights::SEND, 4)?;
    let recv_cap = capabilities::create(
        CapabilityKind::IpcEndpoint { endpoint_id: id.0 }, Rights::RECEIVE, 0)?;
    Some((id, send_cap, recv_cap))
}

pub fn send(cap: CapabilityId, endpoint: EndpointId, msg: Message) -> Result<(), IpcError> {
    if !capabilities::check(cap, Rights::SEND) { return Err(IpcError::PermissionDenied); }
    let mut reg = REGISTRY.lock();
    reg.get_mut(endpoint)
        .ok_or(IpcError::InvalidEndpoint)?
        .push(msg)
        .then_some(())
        .ok_or(IpcError::QueueFull)
}

pub fn recv(cap: CapabilityId, endpoint: EndpointId) -> Result<Message, IpcError> {
    if !capabilities::check(cap, Rights::RECEIVE) { return Err(IpcError::PermissionDenied); }
    let mut reg = REGISTRY.lock();
    reg.get_mut(endpoint)
        .ok_or(IpcError::InvalidEndpoint)?
        .pop()
        .ok_or(IpcError::WouldBlock)
}

pub fn destroy(cap: CapabilityId, endpoint: EndpointId) -> Result<(), IpcError> {
    if !capabilities::check(cap, Rights::REVOKE) {
        return Err(IpcError::PermissionDenied);
    }
    REGISTRY.lock().destroy(endpoint);
    capabilities::revoke(cap);
    Ok(())
}

pub fn send_unchecked(endpoint: EndpointId, msg: Message,) -> Result<(), IpcError> {
    let mut reg = REGISTRY.lock();
    reg.get_mut(endpoint)
        .ok_or(IpcError::InvalidEndpoint)?
        .push(msg)
        .then_some(())
        .ok_or(IpcError::QueueFull)
}

pub fn recv_unchecked(endpoint: EndpointId) -> Result<Message, IpcError> {
    let mut reg = REGISTRY.lock();
    reg.get_mut(endpoint)
        .ok_or(IpcError::InvalidEndpoint)?
        .pop()
        .ok_or(IpcError::WouldBlock)
}