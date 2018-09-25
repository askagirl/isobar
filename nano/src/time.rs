use std::cmp::{self, Ordering};
use std::collections::HashMap;
use std::ops::{Add, AddAssign};
use std::sync::Arc;
use ReplicaId;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Local {
    pub replica_id: ReplicaId,
    pub seq: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Global(Arc<HashMap<ReplicaId, u64>>);

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Lamport {
    pub value: u64,
    pub replica_id: ReplicaId,
}

impl Local {
    pub fn new(replica_id: u64) -> Self {
        Self { replica_id, seq: 1 }
    }

    pub fn tick(&mut self) -> Self {
        let timestamp = *self;
        self.seq += 1;
        timestamp
    }
}

impl Default for Local {
    fn default() -> Self {
        Local {
            replica_id: 0,
            seq: 0,
        }
    }
}

impl<'a> Add<&'a Self> for Local {
    type Output = Local;

    fn add(self, other: &'a Self) -> Self::Output {
        cmp::max(&self, other).clone()
    }
}

impl<'a> AddAssign<&'a Local> for Local {
    fn add_assign(&mut self, other: &Self) {
        if *self < *other {
            *self = other.clone();
        }
    }
}

impl Global {
    pub fn new() -> Self {
        Global(Arc::new(HashMap::new()))
    }

    pub fn get(&self, replica_id: ReplicaId) -> u64 {
        *self.0.get(&replica_id).unwrap_or(&0)
    }

    pub fn observe(&mut self, timestamp: Local) {
        let map = Arc::make_mut(&mut self.0);
        let seq = map.entry(timestamp.replica_id).or_insert(0);
        *seq = cmp::max(*seq, timestamp.seq);
    }

    pub fn observed(&self, timestamp: Local) -> bool {
        self.get(timestamp.replica_id) >= timestamp.seq
    }
}

impl PartialOrd for Global {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut global_ordering = Ordering::Equal;

        for replica_id in self.0.keys().chain(other.0.keys()) {
            let ordering = self.get(*replica_id).cmp(&other.get(*replica_id));
            if ordering != Ordering::Equal {
                if global_ordering == Ordering::Equal {
                    global_ordering = ordering;
                } else if ordering != global_ordering {
                    return None;
                }
            }
        }

        Some(global_ordering)
    }
}

impl Lamport {
    pub fn new(replica_id: ReplicaId) -> Self {
        Self {
            value: 1,
            replica_id,
        }
    }

    pub fn max_value() -> Self {
        Self {
            value: u64::max_value(),
            replica_id: ReplicaId::max_value(),
        }
    }

    pub fn min_value() -> Self {
        Self {
            value: u64::min_value(),
            replica_id: ReplicaId::min_value(),
        }
    }

    pub fn tick(&mut self) -> Self {
        let timestamp = *self;
        self.value += 1;
        timestamp
    }

    pub fn observe(&mut self, timestamp: Self) {
        if timestamp.replica_id != self.replica_id {
            self.value = cmp::max(self.value, timestamp.value) + 1;
        }
    }
}
