import assert from "node:assert/strict";
import test from "node:test";
import { executeVerifiedEntrypoint, VerifiedEntrypointError } from "../dist/verifiedEntrypoint.js";

const runtimeRequest = {
  kind: "runtime_request",
  schema_version: "1.0.0",
  request_id: "browser-demo-001",
  intent: { capability_id: "demo.greet", capability_version: "1.0.0" },
  input: { name: "Ada" },
  lookup: { scope: "public_only", allow_ambiguity: false },
  context: { requested_target: "local" },
  governing_spec: "006-runtime-request-execution",
};

test("executes an exact capability through the verified-entrypoint boundary", async () => {
  let observed;
  const response = await executeVerifiedEntrypoint(async (url, init) => {
    observed = { url, init };
    return new Response(JSON.stringify({ status: "completed", request_id: "browser-demo-001", execution_id: "exec-001", trace_ref: "trace-001", output: { greeting: "hello" }, error: null, trace: { status: "completed" } }), { status: 200 });
  }, "http://127.0.0.1:8787/", { id: "demo.greet", version: "1.0.0", request: runtimeRequest });
  assert.equal(observed.url, "http://127.0.0.1:8787/v1/entrypoints/execute");
  assert.deepEqual(JSON.parse(observed.init.body), { entrypoint_kind: "capability", id: "demo.greet", version: "1.0.0", request: runtimeRequest });
  assert.equal(response.trace_ref, "trace-001");
});

test("surfaces the server's safe verification failure", async () => {
  await assert.rejects(
    () => executeVerifiedEntrypoint(async () => new Response(JSON.stringify({ traverse_code: "verified_entrypoint_not_found", detail: "no verified entrypoint matches" }), { status: 404 }), "http://localhost:8787", { id: "demo.greet", version: "1.0.0", request: runtimeRequest }),
    (error) => error instanceof VerifiedEntrypointError && error.code === "verified_entrypoint_not_found",
  );
});

test("rejects a missing exact identity before making a request", async () => {
  await assert.rejects(
    () => executeVerifiedEntrypoint(async () => { throw new Error("must not fetch"); }, "http://localhost:8787", { id: "", version: "1.0.0", request: runtimeRequest }),
    (error) => error instanceof VerifiedEntrypointError && error.code === "invalid_entrypoint_request",
  );
});
