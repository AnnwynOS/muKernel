use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rights(pub u32);

impl Rights {
    pub const NONE: Rights = Rights(0);
    pub const READ: Rights = Rights(1 << 0);
    pub const WRITE:Rights = Rights(1 << 1);
    pub const EXECUTE:Rights = Rights(1 << 2);
    pub const SEND:Rights = Rights(1 << 3);
    pub const RECEIVE:Rights = Rights(1 << 4);
    pub const GRANT:Rights = Rights(1 << 5);
    pub const REVOKE:Rights = Rights(1 << 6);

    pub fn contains(self, other: Rights) -> bool { (self.0 & other.0) == other.0 }
    pub fn intersect(self, other: Rights) -> Rights { Rights(self.0 & other.0) }
}

impl core::ops::BitOr for Rights {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { Rights(self.0 | rhs.0) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityKind {
    PhysicalMemory { base: u64, size: u64 },
    IpcEndpoint { endpoint_id: u64 },
    StorageObject { object_id: u64 },
    GpuContext { context_id: u32 },
    Service { service_id: u32 },
    Irq { irq_number: u8 },
}

#[derive(Clone, Copy)]
struct Entry {
    id: u64,
    kind: CapabilityKind,
    rights: Rights,
    delegation_depth: u8,
    revoked: bool,
    used: bool,
}

impl Entry {
    const fn empty() -> Self {
        Self {
            id: 0,
            kind: CapabilityKind::Service { service_id: 0 },
            rights: Rights::NONE,
            delegation_depth: 0,
            revoked: false,
            used: false,
        }
    }
}

const MAX_CAPS: usize = 64;

struct Table {
    entries: [Entry; MAX_CAPS],
    next_id: u64,
    count: usize,
}

impl Table {
    const fn new() -> Self {
        Self {
            entries: [Entry::empty(); MAX_CAPS],
            next_id: 1,
            count: 0,
        }
    }

    fn insert(&mut self, kind: CapabilityKind, rights: Rights, depth: u8) -> Option<CapabilityId> {
        if self.count >= MAX_CAPS { return None; }
        for e in self.entries.iter_mut() {
            if !e.used {
                e.id = self.next_id;
                e.kind = kind;
                e.rights = rights;
                e.delegation_depth = depth;
                e.revoked = false;
                e.used = true;
                self.next_id += 1;
                self.count += 1;
                return Some(CapabilityId(e.id));
            }
        }
        None
    }

    fn get(&self, id: CapabilityId) -> Option<&Entry> {
        self.entries.iter().find(|e| e.used && !e.revoked && e.id == id.0)
    }

    fn revoke(&mut self, id: CapabilityId) -> bool {
        for e in self.entries.iter_mut() {
            if e.used && e.id == id.0 {
                e.revoked = true;
                self.count -= 1;
                return true;
            }
        }
        false
    }
}
static TABLE: Mutex<Table> = Mutex::new(Table::new());

pub fn create(kind: CapabilityKind, rights: Rights, delegation_depth: u8) -> Option<CapabilityId> {
    TABLE.lock().insert(kind, rights, delegation_depth)
}

pub fn check(id: CapabilityId, required: Rights) -> bool {
    TABLE.lock().get(id).map(|e| e.rights.contains(required)).unwrap_or(false)
}

pub fn revoke(id: CapabilityId) -> bool {
    TABLE.lock().revoke(id)
}

pub fn delegate(parent: CapabilityId, reduced_rights: Rights) -> Option<CapabilityId> {
    let mut t = TABLE.lock();
    let (kind, depth, actual) = {
        let e = t.get(parent)?;
        if e.delegation_depth == 0 { return None; }
        (e.kind, e.delegation_depth - 1, e.rights.intersect(reduced_rights))
    };
    t.insert(kind, actual, depth)
}