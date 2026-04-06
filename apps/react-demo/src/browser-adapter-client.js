(function (root, factory) {
  const client = factory();

  if (typeof module === "object" && module.exports) {
    module.exports = client;
  }

  if (root) {
    root.TraverseReactDemoClient = client;
  }
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  const APPROVED_BROWSER_DEMO_SESSION = {
    title: "Plan Expedition",
    summary:
      "Traverse evaluates the governed expedition workflow and assembles a final expedition plan.",
    request: {
      goal: "Plan a two-day alpine expedition for a four-person team.",
      requested_target: "local",
      caller: "browser_demo",
    },
    request_id: "expedition-plan-request-001",
    execution_id: "exec_expedition-plan-request-001",
    trace_id: "trace_exec_expedition-plan-request-001",
  };

  function createLiveDemoState() {
    return {
      phase: "idle",
      statusLabel: "ready",
      streamBanner: "No subscription active yet. Submit the approved request to begin.",
      requestId: null,
      executionId: null,
      stateUpdates: [],
      liveTrace: null,
      liveResult: null,
      error: "",
    };
  }

  function buildApprovedSubscriptionRequest() {
    return {
      subscription_request: {
        kind: "browser_runtime_subscription_request",
        schema_version: "1.0.0",
        governing_spec: "013-browser-runtime-subscription",
        request_id: APPROVED_BROWSER_DEMO_SESSION.request_id,
      },
    };
  }

  function humanizeName(value) {
    return value
      .toString()
      .split("_")
      .filter(Boolean)
      .map((part) => part.slice(0, 1).toUpperCase() + part.slice(1))
      .join(" ");
  }

  function formatDetailValue(value) {
    if (value === null || value === undefined) {
      return "";
    }
    if (typeof value === "string") {
      return value;
    }
    if (typeof value === "number" || typeof value === "boolean") {
      return String(value);
    }
    return JSON.stringify(value);
  }

  function describeStateEvent(stateEvent) {
    const details = stateEvent.details || {};
    const parts = [];

    if (details.transition_reason) {
      parts.push(humanizeName(details.transition_reason));
    }

    for (const [key, value] of Object.entries(details)) {
      if (key === "transition_reason") {
        continue;
      }
      parts.push(`${humanizeName(key)}: ${formatDetailValue(value)}`);
    }

    return parts.length > 0 ? parts.join(" · ") : "State update received.";
  }

  function formatStateUpdate(stateEvent) {
    return {
      state: stateEvent.state,
      title: humanizeName(stateEvent.state),
      timestamp: stateEvent.entered_at,
      detail: describeStateEvent(stateEvent),
    };
  }

  function normalizeSubscriptionMessage(message) {
    const variant = Object.keys(message || {})[0];
    if (!variant) {
      return null;
    }
    return {
      variant,
      payload: message[variant],
    };
  }

  function parseSubscriptionFrame(frame) {
    let eventName = "";
    let data = "";

    for (const line of frame.split(/\r?\n/)) {
      if (line.startsWith("event: ")) {
        eventName = line.slice("event: ".length);
      } else if (line.startsWith("data: ")) {
        data += line.slice("data: ".length);
      }
    }

    if (!eventName || !data) {
      return null;
    }

    return {
      event: eventName,
      data: JSON.parse(data),
    };
  }

  function parseSubscriptionFrames(text) {
    return text
      .split(/\r?\n\r?\n/)
      .map((frame) => frame.trim())
      .filter(Boolean)
      .map(parseSubscriptionFrame)
      .filter(Boolean);
  }

  async function runLiveBrowserSubscription({
    baseUrl = "",
    fetchImpl = globalThis.fetch,
    onMessage,
  } = {}) {
    const adapterPrefix = baseUrl.replace(/\/$/, "");
    const createResponse = await fetchImpl(`${adapterPrefix}/local/browser-subscriptions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(buildApprovedSubscriptionRequest()),
    });

    const createdPayload = await createResponse.text();
    if (!createResponse.ok) {
      throw new Error(`local browser adapter setup failed: ${createdPayload}`);
    }

    const created = JSON.parse(createdPayload);
    const streamResponse = await fetchImpl(`${adapterPrefix}${created.stream_url}`, {
      headers: {
        Accept: "text/event-stream",
      },
    });

    if (!streamResponse.ok) {
      const errorPayload = await streamResponse.text();
      throw new Error(`local browser adapter stream failed: ${errorPayload}`);
    }

    if (!streamResponse.body || typeof streamResponse.body.getReader !== "function") {
      throw new Error("local browser adapter stream did not expose a readable body");
    }

    const reader = streamResponse.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    const messages = [];

    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }

      buffer += decoder.decode(value, { stream: true });
      const frames = buffer.split(/\r?\n\r?\n/);
      buffer = frames.pop() || "";

      for (const frame of frames) {
        const parsed = parseSubscriptionFrame(frame.trim());
        if (!parsed) {
          continue;
        }

        const normalized = normalizeSubscriptionMessage(parsed.data);
        if (!normalized) {
          continue;
        }

        messages.push(normalized);
        if (typeof onMessage === "function") {
          onMessage(normalized, created);
        }
      }
    }

    const tail = buffer.trim();
    if (tail) {
      const parsed = parseSubscriptionFrame(tail);
      if (parsed) {
        const normalized = normalizeSubscriptionMessage(parsed.data);
        if (normalized) {
          messages.push(normalized);
          if (typeof onMessage === "function") {
            onMessage(normalized, created);
          }
        }
      }
    }

    return {
      created,
      messages,
    };
  }

  function applyBrowserSubscriptionMessage(state, message, created) {
    const nextState = {
      ...state,
      error: "",
    };

    switch (message.variant) {
      case "Lifecycle": {
        const lifecycle = message.payload;
        nextState.requestId = lifecycle.request_id;
        nextState.executionId = lifecycle.execution_id;
        nextState.statusLabel =
          lifecycle.status === "subscription_established"
            ? "streaming"
            : lifecycle.status === "stream_completed"
              ? "completed"
              : lifecycle.status;
        nextState.streamBanner =
          lifecycle.status === "subscription_established"
            ? "Subscription established. Streaming ordered runtime updates."
            : "Stream completed. Final trace artifact is now visible.";
        nextState.phase =
          lifecycle.status === "subscription_established"
            ? "streaming"
            : lifecycle.status === "stream_completed"
              ? "completed"
              : nextState.phase;
        if (created) {
          nextState.subscriptionId = created.subscription_id;
        }
        return nextState;
      }
      case "State": {
        nextState.phase = "streaming";
        nextState.statusLabel = "streaming";
        nextState.streamBanner = "Subscription established. Streaming ordered runtime updates.";
        nextState.stateUpdates = nextState.stateUpdates.concat(formatStateUpdate(message.payload.state_event));
        return nextState;
      }
      case "TraceArtifact": {
        nextState.liveTrace = message.payload.trace;
        return nextState;
      }
      case "StreamTerminal": {
        nextState.liveResult = message.payload.result;
        nextState.phase = "completed";
        nextState.statusLabel = "completed";
        nextState.streamBanner = "Stream completed. Final trace artifact is now visible.";
        return nextState;
      }
      case "Error": {
        nextState.phase = "error";
        nextState.statusLabel = "error";
        nextState.error = message.payload.message;
        nextState.streamBanner = message.payload.message;
        return nextState;
      }
      default:
        return nextState;
    }
  }

  function traceSummary(trace, terminalResult) {
    if (!trace || !trace.selection || !trace.execution) {
      return null;
    }

    const output = (terminalResult && terminalResult.output) || (trace.result && trace.result.output) || null;
    return {
      selection: {
        capability: trace.selection.selected_capability_id,
        version: trace.selection.selected_capability_version,
        placementTarget: trace.execution.placement.selected_target,
        placementReason: trace.execution.placement.reason,
      },
      emittedEvents: trace.emitted_events || [],
      output: output
        ? {
            planId: output.plan_id,
            route: output.route,
            weatherSummary: output.weather_summary,
            teamStatus: output.team_status,
            nextAction: output.next_action,
          }
        : null,
    };
  }

  return {
    APPROVED_BROWSER_DEMO_SESSION,
    applyBrowserSubscriptionMessage,
    buildApprovedSubscriptionRequest,
    createLiveDemoState,
    humanizeName,
    normalizeSubscriptionMessage,
    parseSubscriptionFrames,
    parseSubscriptionFrame,
    runLiveBrowserSubscription,
    traceSummary,
  };
});
