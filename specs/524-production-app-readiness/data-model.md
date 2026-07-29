# Data Model

| Entity | Required fields | Rules |
| --- | --- | --- |
| RegistryTier | tier, publisher, support state | Certified is default-visible; Community/Kit require opt-in. |
| ResolvedLockGeneration | app id, exact version, artifact digest, publisher, tier, source release/index digest | Immutable after preparation; active generation is explicit. |
| VerifiedCacheGeneration | generation id, lock digest, verified entries, activation state | Only verified active entries may initialize embedded runtime. |
| SecurityYankPolicy | artifact identity, minimum-safe-version and/or deadline | Blocks Certified execution only when locally known and applicable. |
| DurableTraceJournal | workspace id, safe trace record, retention state, prune evidence | Never contains raw request/output payloads. |
| DataStoreMigration | source/target format, backup identity, verification result, restore outcome | Host-directed and single-writer only. |
