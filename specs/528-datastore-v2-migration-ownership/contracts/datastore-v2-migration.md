# DataStore v2 Migration Boundary Contracts

- `datastore.migrate(root_handle, approved_transition, backup_handle)` performs
  a host-authorized `local-datastore/1` to `local-datastore/2` migration and
  returns safe counts, format versions, backup evidence, and a stable outcome.
- `datastore.restore(root_handle, backup_handle)` is explicit, verifies the
  backup binding before it writes, and never discovers a root or backup path.
- `datastore.acquire_owner(root_handle, owner_token)` allows exactly one local
  writer; another writer receives `datastore_owner_locked` without root detail.
