using System.Security.Cryptography;

namespace Traverse.Embedder;

/// <summary>Stable, secret-free errors for the host-owned registry cache.</summary>
public sealed class RegistryCacheException(string code, string message) : Exception(message)
{
    public string Code { get; } = code;
}

/// <summary>Non-secret evidence retained for one verified registry dependency.</summary>
public sealed record RegistryCacheEvidence(
    string Namespace, string Id, string SelectedVersion, string VersionRange,
    string SourceRelease, string IndexDigest, string ArtifactDigest, long VerifiedAt,
    string Outcome);

/// <summary>A selected public record and its host-fetched artifact bytes.</summary>
public sealed record RegistryCachePreparation(
    string Namespace, string Id, string SelectedVersion, string VersionRange,
    string SourceRelease, string IndexDigest, string ArtifactDigest, byte[] ArtifactBytes);

/// <summary>Host-owned cache adapter. It never performs network I/O.</summary>
public sealed class InMemoryRegistryCache
{
    private readonly Dictionary<string, (byte[] Bytes, RegistryCacheEvidence Evidence)> entries = [];

    public RegistryCacheEvidence Prepare(RegistryCachePreparation input)
    {
        if (!MatchesDigest(input.ArtifactBytes, input.ArtifactDigest))
            throw new RegistryCacheException("registry_artifact_digest_mismatch", "registry artifact bytes do not match the published digest");
        var evidence = new RegistryCacheEvidence(input.Namespace, input.Id, input.SelectedVersion,
            input.VersionRange, input.SourceRelease, input.IndexDigest, input.ArtifactDigest,
            DateTimeOffset.UtcNow.ToUnixTimeSeconds(), "prepared");
        entries[Key(input.Namespace, input.Id, input.VersionRange)] = (input.ArtifactBytes.ToArray(), evidence);
        return evidence;
    }

    public (byte[] ArtifactBytes, RegistryCacheEvidence Evidence) ResolveOffline(string @namespace, string id, string versionRange)
    {
        if (!entries.TryGetValue(Key(@namespace, id, versionRange), out var entry))
            throw new RegistryCacheException("registry_cache_entry_missing", "verified registry cache entry is missing for registry_ref");
        if (!MatchesDigest(entry.Bytes, entry.Evidence.ArtifactDigest))
            throw new RegistryCacheException("registry_artifact_digest_mismatch", "cached registry artifact digest mismatch");
        return (entry.Bytes.ToArray(), entry.Evidence with { Outcome = "resolved" });
    }

    private static string Key(string @namespace, string id, string range) => $"{@namespace}:{id}:{range}";
    private static bool MatchesDigest(byte[] bytes, string digest) =>
        digest == "sha256:" + Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
}
