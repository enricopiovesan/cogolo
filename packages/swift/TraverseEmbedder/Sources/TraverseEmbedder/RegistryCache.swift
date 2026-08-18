import CryptoKit
import Foundation

/// Stable, secret-free failure for host-owned registry cache operations.
public struct RegistryCacheError: Error, Equatable, Sendable { public let code: String; public let message: String }
public struct RegistryCacheEvidence: Sendable, Equatable {
    public let namespace: String; public let id: String; public let selectedVersion: String; public let versionRange: String
    public let sourceRelease: String; public let indexDigest: String; public let artifactDigest: String; public let verifiedAt: Int; public let outcome: String
}
public struct RegistryCachePreparation: Sendable {
    public let namespace: String; public let id: String; public let selectedVersion: String; public let versionRange: String
    public let sourceRelease: String; public let indexDigest: String; public let artifactDigest: String; public let artifactBytes: Data
}

/// Host-owned in-memory cache adapter. It deliberately performs no network I/O.
public final class InMemoryRegistryCache: @unchecked Sendable {
    private var entries: [String: (Data, RegistryCacheEvidence)] = [:]
    public init() {}
    public func prepare(_ input: RegistryCachePreparation) throws -> RegistryCacheEvidence {
        guard matches(input.artifactBytes, input.artifactDigest) else { throw RegistryCacheError(code: "registry_artifact_digest_mismatch", message: "registry artifact bytes do not match the published digest") }
        let evidence = RegistryCacheEvidence(namespace: input.namespace, id: input.id, selectedVersion: input.selectedVersion, versionRange: input.versionRange, sourceRelease: input.sourceRelease, indexDigest: input.indexDigest, artifactDigest: input.artifactDigest, verifiedAt: Int(Date().timeIntervalSince1970), outcome: "prepared")
        entries[key(input.namespace, input.id, input.versionRange)] = (input.artifactBytes, evidence); return evidence
    }
    public func resolveOffline(namespace: String, id: String, versionRange: String) throws -> (Data, RegistryCacheEvidence) {
        guard let entry = entries[key(namespace, id, versionRange)] else { throw RegistryCacheError(code: "registry_cache_entry_missing", message: "verified registry cache entry is missing for registry_ref") }
        guard matches(entry.0, entry.1.artifactDigest) else { throw RegistryCacheError(code: "registry_artifact_digest_mismatch", message: "cached registry artifact digest mismatch") }
        return (entry.0, RegistryCacheEvidence(namespace: entry.1.namespace, id: entry.1.id, selectedVersion: entry.1.selectedVersion, versionRange: entry.1.versionRange, sourceRelease: entry.1.sourceRelease, indexDigest: entry.1.indexDigest, artifactDigest: entry.1.artifactDigest, verifiedAt: entry.1.verifiedAt, outcome: "resolved"))
    }
    private func key(_ namespace: String, _ id: String, _ range: String) -> String { "\(namespace):\(id):\(range)" }
    private func matches(_ bytes: Data, _ digest: String) -> Bool { digest == "sha256:" + SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined() }
}
