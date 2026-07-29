import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";
import {
  MemoryRegistryCacheStore,
  RegistryCacheError,
  prepareRegistryDependency,
  resolveRegistryDependencyOffline,
} from "../dist/registryCache.js";

function digestFor(bytes) {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function sampleSnapshot(deprecated = false) {
  const artifact = Buffer.from("wasm-bytes");
  const contract = Buffer.from('{"kind":"capability_contract"}');
  const record = {
    namespace: "demo",
    id: "greet",
    version: "1.2.0",
    digest: digestFor(artifact),
    artifactUrl: "https://example.test/greet.wasm",
    contractDigest: digestFor(contract),
    contractUrl: "https://example.test/greet.json",
    deprecated,
  };
  const assets = new Map([
    [record.artifactUrl, new Uint8Array(artifact)],
    [record.contractUrl, new Uint8Array(contract)],
  ]);
  return {
    snapshot: {
      releaseTag: "index-v9",
      capabilities: [
        {
          namespace: "demo",
          id: "greet",
          version: "1.0.0",
          digest: digestFor(Buffer.from("older")),
          artifactUrl: "https://example.test/older.wasm",
          contractDigest: digestFor(Buffer.from("older-contract")),
          contractUrl: "https://example.test/older.json",
          deprecated: false,
        },
        record,
      ],
    },
    assets,
    reference: { namespace: "demo", id: "greet", versionRange: "^1.0.0" },
  };
}

test("prepare then offline resolve round trip", async () => {
  const store = new MemoryRegistryCacheStore();
  const { snapshot, assets, reference } = sampleSnapshot(false);
  const evidence = await prepareRegistryDependency(store, snapshot, reference, {
    fetch(url) {
      const bytes = assets.get(url);
      if (!bytes) {
        throw new Error("missing");
      }
      return bytes;
    },
  });
  assert.equal(evidence.selectedVersion, "1.2.0");
  const resolved = await resolveRegistryDependencyOffline(store, reference);
  assert.equal(resolved.evidence.selectedVersion, "1.2.0");
  assert.deepEqual(Buffer.from(resolved.wasmBytes), Buffer.from("wasm-bytes"));
});

test("offline resolve without prepare is missing", async () => {
  const store = new MemoryRegistryCacheStore();
  await assert.rejects(
    () =>
      resolveRegistryDependencyOffline(store, {
        namespace: "demo",
        id: "greet",
        versionRange: "^1.0.0",
      }),
    (error) =>
      error instanceof RegistryCacheError &&
      error.code === "registry_cache_entry_missing",
  );
});

test("yanked-only range fails closed", async () => {
  const store = new MemoryRegistryCacheStore();
  const { snapshot, assets, reference } = sampleSnapshot(true);
  snapshot.capabilities = snapshot.capabilities.filter(
    (record) => record.version === "1.2.0",
  );
  await assert.rejects(
    () =>
      prepareRegistryDependency(store, snapshot, reference, {
        fetch(url) {
          return assets.get(url);
        },
      }),
    (error) =>
      error instanceof RegistryCacheError &&
      error.code === "registry_dependency_yanked",
  );
});

test("digest mismatch leaves no usable entry", async () => {
  const store = new MemoryRegistryCacheStore();
  const { snapshot, assets, reference } = sampleSnapshot(false);
  assets.set(
    "https://example.test/greet.wasm",
    new Uint8Array(Buffer.from("tampered")),
  );
  await assert.rejects(
    () =>
      prepareRegistryDependency(store, snapshot, reference, {
        fetch(url) {
          return assets.get(url);
        },
      }),
    (error) =>
      error instanceof RegistryCacheError &&
      error.code === "registry_artifact_digest_mismatch",
  );
  await assert.rejects(
    () => resolveRegistryDependencyOffline(store, reference),
    (error) =>
      error instanceof RegistryCacheError &&
      error.code === "registry_cache_entry_missing",
  );
});
