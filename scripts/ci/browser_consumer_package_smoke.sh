#!/usr/bin/env bash

set -euo pipefail

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
const consumer = require('./apps/browser-consumer');

(async () => {
  assert.equal(
    consumer.APPROVED_BROWSER_CONSUMER_SESSION.title,
    'Traverse Browser Consumer',
  );

  const createAndStream = await consumer.runBrowserConsumerSubscription({
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

  const terminalMessage = createAndStream.messages.find((message) => message.variant === 'StreamTerminal');
  assert.ok(terminalMessage);
  assert.equal(terminalMessage.payload.result.status, 'completed');

  const traceMessage = createAndStream.messages.find((message) => message.variant === 'TraceArtifact');
  assert.ok(traceMessage);

  const summary = consumer.browserConsumerTraceSummary(
    traceMessage.payload.trace,
    terminalMessage.payload.result,
  );
  assert.ok(summary);
  assert.equal(summary.selection.capability, 'expedition.planning.plan-expedition');
  assert.equal(summary.output.planId, 'plan-objective-skypilot');
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
NODE

popd >/dev/null
