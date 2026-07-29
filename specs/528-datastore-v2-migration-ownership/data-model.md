# Data Model

| Entity | Required fields | Invariants |
| --- | --- | --- |
| `DataStoreV2Envelope` | format id, format version, schema version, record digest, payload envelope, integrity metadata | Is file-backed, self-identifies as `local-datastore/2`, and does not disclose root or key material in evidence. |
| `MigrationBackup` | source/target format, source digest, backup digest, created-at, verification result | Binds exactly one verified source representation to its approved transition. |
| `MigrationReport` | source/target version, record counts, backup evidence id, outcome | Is safe to retain; contains no stored values, roots, or encryption keys. |
| `OwnerLease` | opaque owner token, acquisition state, lifecycle evidence | Has one active writer for a root; it is host-scoped and never serialized in the store. |
