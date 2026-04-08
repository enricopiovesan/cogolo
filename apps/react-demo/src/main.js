const { createElement: h, useState } = React;

const DemoClient = window.TraverseReactDemoClient;
const APPROVED_SESSION = DemoClient.APPROVED_BROWSER_DEMO_SESSION;

function labeledCard(label, value) {
  return h(
    "div",
    null,
    h("dt", null, label),
    h("dd", null, value),
  );
}

function timelineItem(update) {
  return h(
    "li",
    {
      key: `${update.timestamp}-${update.state}`,
      className: "timeline-item",
    },
    h("div", { className: "timeline-marker" }, update.state.slice(0, 1).toUpperCase()),
    h(
      "div",
      { className: "timeline-body" },
      h(
        "div",
        { className: "timeline-topline" },
        h("strong", null, update.title),
        h("span", null, update.timestamp),
      ),
      h("p", null, update.detail),
    ),
  );
}

function liveTraceSection(trace, terminalResult) {
  const summary = DemoClient.traceSummary(trace, terminalResult);

  if (!summary) {
    return h(
      "div",
      { className: "trace-placeholder" },
      "Terminal trace is withheld until the ordered state stream reaches completion.",
    );
  }

  return [
    h(
      "div",
      { className: "trace-block", key: "selection" },
      h("h3", null, "Selection"),
      h(
        "dl",
        { className: "trace-list" },
        labeledCard("Capability", summary.selection.capability),
        labeledCard("Version", summary.selection.version),
        labeledCard(
          "Placement",
          `${summary.selection.placementTarget} · ${summary.selection.placementReason}`,
        ),
      ),
    ),
    h(
      "div",
      { className: "trace-block", key: "events" },
      h("h3", null, "Emitted Events"),
      h(
        "ul",
        { className: "event-list" },
        summary.emittedEvents.map((eventId) => h("li", { key: eventId }, eventId)),
      ),
    ),
    h(
      "div",
      { className: "trace-block", key: "output" },
      h("h3", null, "Output"),
      h(
        "dl",
        { className: "trace-list" },
        labeledCard("Plan", summary.output.planId),
        labeledCard("Route", summary.output.route),
        labeledCard("Weather", summary.output.weatherSummary),
        labeledCard("Team Status", summary.output.teamStatus),
        labeledCard("Next Action", summary.output.nextAction),
      ),
    ),
  ];
}

function DemoApp() {
  const [sessionState, setSessionState] = useState(() => DemoClient.createLiveDemoState());
  const [error, setError] = useState("");

  async function handleSubmitRequest() {
    if (sessionState.phase === "streaming") {
      return;
    }

    setError("");
    setSessionState(DemoClient.createLiveDemoState());

    try {
      await DemoClient.runLiveBrowserSubscription({
        onMessage: (message, created) => {
          setSessionState((current) => DemoClient.applyBrowserSubscriptionMessage(current, message, created));
        },
      });
    } catch (reason) {
      const message = String(reason);
      setError(message);
      setSessionState((current) => ({
        ...current,
        phase: "error",
        statusLabel: "error",
        streamBanner: message,
        error: message,
      }));
    }
  }

  const statusLabel = sessionState.statusLabel;
  const hasTerminalTrace = sessionState.phase === "completed" && sessionState.liveTrace;
  const isStreaming = sessionState.phase === "streaming";
  const visibleUpdates = sessionState.stateUpdates;

  if (error) {
    return h(
      "main",
      { className: "page" },
      h(
        "section",
        { className: "hero" },
        h(
          "div",
          { className: "hero-copy" },
          h("p", { className: "eyebrow" }, "Traverse Browser Runtime"),
          h("h1", null, "Live adapter connection failed."),
          h("p", { className: "lede" }, error),
          h(
            "p",
            { className: "lede" },
            "Run the local browser adapter proxy again, or use the documented fixture preview fallback.",
          ),
        ),
      ),
    );
  }

  return h(
    "main",
    { className: "page" },
    h(
      "section",
      { className: "hero" },
      h(
        "div",
        { className: "hero-copy" },
        h("p", { className: "eyebrow" }, "Traverse Browser Runtime"),
        h("h1", null, APPROVED_SESSION.title),
        h("p", { className: "lede" }, APPROVED_SESSION.summary),
        h(
          "dl",
          { className: "request-meta" },
          labeledCard("Goal", APPROVED_SESSION.request.goal),
          labeledCard("Target", APPROVED_SESSION.request.requested_target),
          labeledCard("Trace", APPROVED_SESSION.trace_id),
          labeledCard("Request", APPROVED_SESSION.request_id),
        ),
      ),
      h("div", { className: "status-pill" }, statusLabel),
    ),
    h(
      "section",
      { className: "grid" },
      h(
        "article",
        { className: "panel" },
        h(
          "div",
          { className: "panel-header" },
          h("h2", null, "Request And Stream"),
          h(
            "p",
            null,
            "Submit the approved expedition request, then watch the governed browser subscription stream unfold in order.",
          ),
        ),
        h(
          "div",
          { className: "request-card" },
          h(
            "div",
            null,
            h("p", { className: "request-label" }, "Approved request"),
            h("h3", null, APPROVED_SESSION.title),
            h("p", null, APPROVED_SESSION.request.goal),
          ),
          h(
            "button",
            {
              className: "request-button",
              type: "button",
              onClick: handleSubmitRequest,
              disabled: isStreaming,
            },
            isStreaming ? "Streaming approved request..." : "Submit approved request",
          ),
        ),
        h("div", { className: "stream-banner" }, sessionState.streamBanner),
        h("ol", { className: "timeline" }, visibleUpdates.map(timelineItem)),
      ),
      h(
        "article",
        { className: "panel trace-panel" },
        h(
          "div",
          { className: "panel-header" },
          h("h2", null, "Terminal Trace"),
          h("p", null, "The final governed selection, placement, and output snapshot."),
        ),
        hasTerminalTrace
          ? liveTraceSection(sessionState.liveTrace, sessionState.liveResult)
          : h(
              "div",
              { className: "trace-placeholder" },
              "Terminal trace is withheld until the ordered state stream reaches completion.",
            ),
      ),
    ),
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(h(DemoApp));
