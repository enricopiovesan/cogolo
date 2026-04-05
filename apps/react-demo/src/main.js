const { createElement: h, useEffect, useState } = React;

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

function traceSection(session) {
  return [
    h(
      "div",
      { className: "trace-block", key: "selection" },
      h("h3", null, "Selection"),
      h(
        "dl",
        { className: "trace-list" },
        labeledCard("Capability", session.trace.selected_capability_id),
        labeledCard("Version", session.trace.selected_capability_version),
        labeledCard(
          "Placement",
          `${session.trace.placement.selected_target} · ${session.trace.placement.reason}`,
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
        session.trace.emitted_events.map((eventId) => h("li", { key: eventId }, eventId)),
      ),
    ),
    h(
      "div",
      { className: "trace-block", key: "output" },
      h("h3", null, "Output"),
      h(
        "dl",
        { className: "trace-list" },
        labeledCard("Plan", session.trace.output.plan_id),
        labeledCard("Route", session.trace.output.route),
        labeledCard("Weather", session.trace.output.weather_summary),
        labeledCard("Team Status", session.trace.output.team_status),
        labeledCard("Next Action", session.trace.output.next_action),
      ),
    ),
  ];
}

function DemoApp() {
  const [session, setSession] = useState(null);
  const [error, setError] = useState("");
  const [phase, setPhase] = useState("loading");
  const [visibleCount, setVisibleCount] = useState(0);

  useEffect(() => {
    fetch("./public/expedition-runtime-session.json")
      .then((response) => {
        if (!response.ok) {
          throw new Error(`failed to load fixture: ${response.status}`);
        }
        return response.json();
      })
      .then((loadedSession) => {
        setSession(loadedSession);
        setPhase("idle");
      })
      .catch((reason) => {
        setError(String(reason));
      });
  }, []);

  useEffect(() => {
    if (!session || phase !== "streaming") {
      return undefined;
    }

    if (visibleCount >= session.state_updates.length) {
      setPhase("completed");
      return undefined;
    }

    const timer = window.setTimeout(() => {
      setVisibleCount((current) => current + 1);
    }, 450);

    return () => window.clearTimeout(timer);
  }, [phase, session, visibleCount]);

  function handleSubmitRequest() {
    setVisibleCount(0);
    setPhase("streaming");
  }

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
          h("h1", null, "Fixture loading failed."),
          h("p", { className: "lede" }, error),
        ),
      ),
    );
  }

  if (!session) {
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
          h("h1", null, "Loading the governed expedition session..."),
          h(
            "p",
            { className: "lede" },
            "Preparing ordered runtime state updates and the terminal trace artifact.",
          ),
        ),
      ),
    );
  }

  const visibleUpdates = session.state_updates.slice(0, visibleCount);
  const hasTerminalTrace = phase === "completed";
  const isStreaming = phase === "streaming";
  const statusLabel =
    phase === "idle"
      ? "ready"
      : phase === "streaming"
        ? "streaming"
        : phase === "completed"
          ? session.status
          : "loading";

  let streamBanner = "No subscription active yet. Submit the approved request to begin.";
  if (phase === "streaming") {
    streamBanner = "Subscription established. Streaming ordered runtime updates.";
  } else if (phase === "completed") {
    streamBanner = "Stream completed. Final trace artifact is now visible.";
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
        h("h1", null, session.title),
        h("p", { className: "lede" }, session.summary),
        h(
          "dl",
          { className: "request-meta" },
          labeledCard("Goal", session.request.goal),
          labeledCard("Target", session.request.requested_target),
          labeledCard("Trace", session.trace_id),
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
            h("h3", null, session.title),
            h("p", null, session.request.goal),
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
        h("div", { className: "stream-banner" }, streamBanner),
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
          ? traceSection(session)
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
