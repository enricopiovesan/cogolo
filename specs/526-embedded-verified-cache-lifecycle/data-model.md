# Data Model

| Entity | Required fields | Rule |
| --- | --- | --- |
| VerifiedCacheGeneration | id, lock digest, entries, evidence, state | Only complete verified candidates may activate. |
| ActiveGenerationPointer | generation id, changed-at, prior id | Changes atomically and never auto-selects. |
| CacheEntryManifest | identity, artifact digest, verification outcome | Does not include bytes, path, or credentials. |
| StaleProvenance | prepared-at, source/index/lifecycle digests, policy outcome | Explains offline state without refresh. |
