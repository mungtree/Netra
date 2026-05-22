//! Injectable support services, so time and identity are controllable in tests.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A source of the current time.
pub trait Clock: Send + Sync {
    /// Returns the current instant in UTC.
    fn now(&self) -> DateTime<Utc>;
}

/// A source of fresh identifiers.
pub trait IdGenerator: Send + Sync {
    /// Returns a new unique id.
    fn generate(&self) -> Uuid;
}

/// The production [`Clock`], reading the system wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// The production [`IdGenerator`], producing random v4 UUIDs.
#[derive(Debug, Clone, Copy, Default)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn generate(&self) -> Uuid {
        Uuid::new_v4()
    }
}
