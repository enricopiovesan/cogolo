package dev.traverse.embedder

import java.security.MessageDigest

/** Stable, secret-free failure for host-owned registry cache operations. */
class RegistryCacheException(val code: String, message: String) : IllegalStateException(message)

/** Non-secret evidence for a verified registry dependency. */
data class RegistryCacheEvidence(
    val namespace: String, val id: String, val selectedVersion: String, val versionRange: String,
    val sourceRelease: String, val indexDigest: String, val artifactDigest: String,
    val verifiedAt: Long, val outcome: String,
)

/** A host-selected, fetched dependency presented for verification and storage. */
data class RegistryCachePreparation(
    val namespace: String, val id: String, val selectedVersion: String, val versionRange: String,
    val sourceRelease: String, val indexDigest: String, val artifactDigest: String, val artifactBytes: ByteArray,
)

/** Host-owned, in-memory cache adapter. It deliberately performs no network I/O. */
class InMemoryRegistryCache {
    private val entries = mutableMapOf<String, Pair<ByteArray, RegistryCacheEvidence>>()

    fun prepare(input: RegistryCachePreparation): RegistryCacheEvidence {
        if (!matches(input.artifactBytes, input.artifactDigest)) throw RegistryCacheException(
            "registry_artifact_digest_mismatch", "registry artifact bytes do not match the published digest")
        val evidence = RegistryCacheEvidence(input.namespace, input.id, input.selectedVersion, input.versionRange,
            input.sourceRelease, input.indexDigest, input.artifactDigest, System.currentTimeMillis() / 1000, "prepared")
        entries[key(input.namespace, input.id, input.versionRange)] = input.artifactBytes.copyOf() to evidence
        return evidence
    }

    fun resolveOffline(namespace: String, id: String, versionRange: String): Pair<ByteArray, RegistryCacheEvidence> {
        val entry = entries[key(namespace, id, versionRange)] ?: throw RegistryCacheException(
            "registry_cache_entry_missing", "verified registry cache entry is missing for registry_ref")
        if (!matches(entry.first, entry.second.artifactDigest)) throw RegistryCacheException(
            "registry_artifact_digest_mismatch", "cached registry artifact digest mismatch")
        return entry.first.copyOf() to entry.second.copy(outcome = "resolved")
    }

    private fun key(namespace: String, id: String, range: String) = "$namespace:$id:$range"
    private fun matches(bytes: ByteArray, digest: String) = digest == "sha256:" +
        MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
}
