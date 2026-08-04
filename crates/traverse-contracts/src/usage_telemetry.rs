//! Provider-neutral usage-telemetry port (spec `088-runtime-usage-telemetry`
//! FR-001). No caller of [`UsageTelemetrySink`] takes on a network or
//! configuration dependency merely by calling it: [`NoOpUsageTelemetrySink`]
//! is the default and performs no I/O of any kind.

/// Which kind of usage event occurred (spec 088 FR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageEventKind {
    /// A registry version lookup resolved successfully.
    Resolve,
    /// A capability was executed via a real WASM invocation.
    Execute,
}

/// One usage-telemetry event: exactly the capability-identifying fields
/// spec 088 FR-004 permits (the install ID, also required by FR-004, is
/// attached by the caller's concrete sink implementation, not this port).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    /// Which kind of event occurred.
    pub kind: UsageEventKind,
    /// The capability reference, formatted as `namespace/id@version`.
    pub capability_ref: String,
    /// Event timestamp (ISO 8601).
    pub timestamp: String,
}

/// Provider-neutral port for recording usage-telemetry events.
///
/// Implementations MUST NOT block or fail the caller on downstream failure:
/// per FR-005, any collector failure (timeout, DNS, non-2xx) must be
/// swallowed entirely by the concrete sink, never surfaced through `record`.
pub trait UsageTelemetrySink: Send + Sync {
    /// Records one usage event.
    fn record(&self, event: UsageEvent);
}

/// Default [`UsageTelemetrySink`]: performs no I/O of any kind (FR-001,
/// FR-007). Wired whenever telemetry is disabled or unconfigured, so no
/// caller (including `crates/traverse-registry`) takes on a network or
/// configuration dependency merely by holding a sink.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpUsageTelemetrySink;

impl UsageTelemetrySink for NoOpUsageTelemetrySink {
    fn record(&self, _event: UsageEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_op_sink_records_without_side_effects() {
        let sink = NoOpUsageTelemetrySink;
        sink.record(UsageEvent {
            kind: UsageEventKind::Resolve,
            capability_ref: "hello.world/say-hello@1.0.0".to_string(),
            timestamp: "2026-08-04T00:00:00Z".to_string(),
        });
        sink.record(UsageEvent {
            kind: UsageEventKind::Execute,
            capability_ref: "hello.world/say-hello@1.0.0".to_string(),
            timestamp: "2026-08-04T00:00:00Z".to_string(),
        });
    }

    #[test]
    #[allow(clippy::clone_on_copy, clippy::default_constructed_unit_structs)]
    fn no_op_sink_default_clone_and_debug_are_stable() {
        let sink = NoOpUsageTelemetrySink::default();
        let cloned = sink.clone();
        assert_eq!(format!("{sink:?}"), format!("{cloned:?}"));
        assert_eq!(format!("{sink:?}"), "NoOpUsageTelemetrySink");
    }

    #[test]
    fn no_op_sink_is_usable_as_a_trait_object() {
        let sink: Box<dyn UsageTelemetrySink> = Box::new(NoOpUsageTelemetrySink);
        sink.record(UsageEvent {
            kind: UsageEventKind::Resolve,
            capability_ref: "a.b/c@1.0.0".to_string(),
            timestamp: "t".to_string(),
        });
    }

    #[test]
    fn usage_event_kind_equality_and_debug() {
        assert_eq!(UsageEventKind::Resolve, UsageEventKind::Resolve);
        assert_ne!(UsageEventKind::Resolve, UsageEventKind::Execute);
        assert_eq!(format!("{:?}", UsageEventKind::Resolve), "Resolve");
        assert_eq!(format!("{:?}", UsageEventKind::Execute), "Execute");
        assert_eq!(UsageEventKind::Execute.clone(), UsageEventKind::Execute);
    }

    #[test]
    fn usage_event_equality_clone_and_debug() {
        let event = UsageEvent {
            kind: UsageEventKind::Resolve,
            capability_ref: "a.b/c@1.0.0".to_string(),
            timestamp: "t".to_string(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
        assert!(format!("{event:?}").contains("Resolve"));

        let different = UsageEvent {
            kind: UsageEventKind::Execute,
            ..event.clone()
        };
        assert_ne!(event, different);
    }
}
