#!/usr/bin/env bash

set -euo pipefail

# scripts/ci/react_demo_live_adapter_smoke.sh verifies the live browser adapter path end to end.

repo_root="${TRAVERSE_REPO_ROOT:-$(pwd)}"
tmpdir="$(mktemp -d)"
adapter_log="${tmpdir}/browser-adapter.log"
demo_log="${tmpdir}/react-demo.log"
adapter_pid=""
demo_pid=""

cleanup() {
  if [[ -n "${demo_pid}" ]] && kill -0 "${demo_pid}" 2>/dev/null; then
    kill "${demo_pid}" 2>/dev/null || true
    wait "${demo_pid}" 2>/dev/null || true
  fi
  if [[ -n "${adapter_pid}" ]] && kill -0 "${adapter_pid}" 2>/dev/null; then
    kill "${adapter_pid}" 2>/dev/null || true
    wait "${adapter_pid}" 2>/dev/null || true
  fi
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

pushd "${repo_root}" >/dev/null

cargo run -p traverse-cli -- browser-adapter serve --bind 127.0.0.1:4174 >"${adapter_log}" 2>&1 &
adapter_pid=$!

for _ in $(seq 1 200); do
  if grep -q "local browser adapter listening on " "${adapter_log}" 2>/dev/null; then
    break
  fi
  if ! kill -0 "${adapter_pid}" 2>/dev/null; then
    cat "${adapter_log}" >&2
    echo "browser adapter exited before it reported a listening address" >&2
    exit 1
  fi
  sleep 0.05
done

node apps/react-demo/server.mjs --adapter http://127.0.0.1:4174 --port 4173 >"${demo_log}" 2>&1 &
demo_pid=$!

for _ in $(seq 1 200); do
  if grep -q "Traverse React demo serving on http://127.0.0.1:4173" "${demo_log}" 2>/dev/null; then
    break
  fi
  if ! kill -0 "${demo_pid}" 2>/dev/null; then
    cat "${demo_log}" >&2
    echo "react demo server exited before it reported a listening address" >&2
    exit 1
  fi
  sleep 0.05
done

node <<'NODE'
const assert = require('node:assert/strict');
const client = require('./apps/react-demo/src/browser-adapter-client.js');

(async () => {
  const root = await fetch('http://127.0.0.1:4173/');
  assert.equal(root.status, 200);
  const html = await root.text();
  assert.match(html, /Traverse React Demo/);

  const createAndStream = await client.runLiveBrowserSubscription({
    baseUrl: 'http://127.0.0.1:4173',
  });

  assert.ok(createAndStream.created.subscription_id);
  assert.ok(createAndStream.messages.length > 0);
  assert.equal(createAndStream.messages[0].variant, 'Lifecycle');
  assert.equal(
    createAndStream.messages[createAndStream.messages.length - 1].variant,
    'Lifecycle',
  );
  assert.ok(createAndStream.messages.some((message) => message.variant === 'State'));
  assert.ok(createAndStream.messages.some((message) => message.variant === 'TraceArtifact'));
  assert.ok(createAndStream.messages.some((message) => message.variant === 'StreamTerminal'));

  const traceMessage = createAndStream.messages.find((message) => message.variant === 'TraceArtifact');
  assert.ok(traceMessage);

  const terminalMessage = createAndStream.messages.find((message) => message.variant === 'StreamTerminal');
  assert.ok(terminalMessage);
  assert.equal(terminalMessage.payload.result.status, 'completed');
  assert.equal(terminalMessage.payload.result.output.plan_id, 'plan-objective-skypilot');

  const trace = traceMessage.payload.trace;
  const summary = client.traceSummary(traceMessage.payload.trace, terminalMessage.payload.result);
  assert.ok(summary);
  assert.equal(summary.selection.capability, 'expedition.planning.plan-expedition');
  assert.equal(summary.output.planId, 'plan-objective-skypilot');

  const invalidResponse = await fetch('http://127.0.0.1:4173/local/browser-subscriptions', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      subscription_request: {
        kind: 'browser_runtime_subscription_request',
        schema_version: '1.0.0',
        governing_spec: '013-browser-runtime-subscription',
      },
    }),
  });
  assert.equal(invalidResponse.status, 400);
  const invalidPayload = await invalidResponse.json();
  assert.equal(invalidPayload.kind, 'local_browser_subscription_setup_error');
  assert.equal(invalidPayload.code, 'invalid_request');

  const missingStream = await fetch('http://127.0.0.1:4173/local/browser-subscriptions/lbs_9999/stream', {
    headers: {
      Accept: 'text/event-stream',
    },
  });
  assert.equal(missingStream.status, 404);
  const missingPayload = await missingStream.json();
  assert.equal(missingPayload.kind, 'local_browser_subscription_stream_error');
  assert.equal(missingPayload.code, 'not_found');
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
NODE

popd >/dev/null
