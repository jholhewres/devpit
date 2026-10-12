//! When a value was written, as every machine agrees to order it: a hybrid
//! logical clock. A stamp is milliseconds, a counter for writes within the
//! same millisecond or behind a clock that went back, and the machine that
//! wrote it, which settles a tie the same way everywhere.
//!
//! The clock never goes backwards: a stamp is after the machine's last one
//! and after every stamp it has seen, whatever the wall clock says.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stamp {
    pub ms: u64,
    pub n: u32,
    /// The machine that wrote it.
    pub by: String,
}

impl Serialize for Stamp {
    fn serialize<S: Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        (self.ms, self.n, &self.by).serialize(out)
    }
}

impl<'de> Deserialize<'de> for Stamp {
    fn deserialize<D: Deserializer<'de>>(read: D) -> Result<Self, D::Error> {
        let (ms, n, by) = <(u64, u32, String)>::deserialize(read)?;
        Ok(Stamp { ms, n, by })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    ms: u64,
    n: u32,
    by: String,
}

impl Clock {
    pub fn new(by: &str) -> Clock {
        Clock {
            ms: 0,
            n: 0,
            by: by.to_owned(),
        }
    }

    /// The stamp for a write made at `wall` milliseconds: after any this
    /// clock gave or saw. Several writes found in one look share the floor
    /// they are measured from, so each keeps its own moment.
    pub fn next(&self, wall: u64) -> Stamp {
        let (ms, n) = if wall > self.ms {
            (wall, 0)
        } else if self.n == u32::MAX {
            (self.ms + 1, 0)
        } else {
            (self.ms, self.n + 1)
        };
        Stamp {
            ms,
            n,
            by: self.by.clone(),
        }
    }

    /// Another machine's stamp: whatever this clock gives next is after it.
    pub fn observe(&mut self, seen: &Stamp) {
        if (seen.ms, seen.n) > (self.ms, self.n) {
            self.ms = seen.ms;
            self.n = seen.n;
        }
    }
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;
