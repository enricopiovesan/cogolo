import assert from "node:assert/strict";
import test from "node:test";

const endpoint = "http://127.0.0.1:8787/v1/workspaces/local-default/apps/traverse-starter/proposals";

async function submit(fetcher, action, proposal) {
  const response = await fetcher(endpoint, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ action, proposal }) });
  return response.json();
}

test("browser conformance: rejected proposal displays structured redacted evidence", async () => {
  const result = await submit(async (_url, init) => {
    assert.equal(JSON.parse(init.body).action, "validate");
    return new Response(JSON.stringify({ valid: false, errors: [{ code: "undeclared_capability", path: "/nodes/0" }] }));
  }, "validate", { kind: "workflow_proposal" });
  assert.equal(result.valid, false);
  assert.equal(result.errors[0].code, "undeclared_capability");
});

test("browser conformance: execution failure remains host-redacted", async () => {
  const result = await submit(async () => new Response(JSON.stringify({ status: "completed", trace: { terminal_state: "failed", node_outcomes: [{ node_id: "a", status: "failed" }] } })), "execute", { kind: "workflow_proposal" });
  assert.equal(result.trace.terminal_state, "failed");
  assert.equal(JSON.stringify(result).includes("planner_prompt"), false);
});
