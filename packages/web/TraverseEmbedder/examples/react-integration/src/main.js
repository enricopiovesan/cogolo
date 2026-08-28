import { BundleEmbedder, FetchBundleLoader } from "/pkg/index.js";

const { createElement: h, useEffect, useState } = React;

const MANIFEST_PATH = "/repo/examples/applications/traverse-starter/app.manifest.json";
const CAPABILITY_ID = "traverse-starter.process";
const PROPOSAL_ENDPOINT = "http://127.0.0.1:8787/v1/workspaces/local-default/apps/traverse-starter/proposals";

function eventList(events) {
  if (events.length === 0) {
    return h("p", null, "No events yet.");
  }
  return h(
    "ol",
    null,
    events.map((event) =>
      h(
        "li",
        { key: event.event_id },
        h("strong", null, event.event_type),
        " — ",
        h("code", null, JSON.stringify(event.data)),
      ),
    ),
  );
}

function App() {
  const [embedder, setEmbedder] = useState(null);
  const [initError, setInitError] = useState(null);
  const [events, setEvents] = useState([]);
  const [note, setNote] = useState("hello from React, no sidecar");
  const [evidence, setEvidence] = useState(null);
  const [proposalText, setProposalText] = useState(JSON.stringify({
    kind: "workflow_proposal", schema_version: "1.0.0", proposal_id: "review-me",
    workspace_id: "local-default", app_manifest: { app_id: "traverse-starter", app_version: "1.0.0", manifest_digest: "host-pinned" },
    nodes: [], edges: [], mappings: [], initial_input: {},
  }, null, 2));
  const [proposalResult, setProposalResult] = useState(null);

  const reviewProposal = async (action) => {
    let proposal;
    try { proposal = JSON.parse(proposalText); } catch (_) { setProposalResult({ status: "invalid JSON — nothing submitted" }); return; }
    const response = await fetch(PROPOSAL_ENDPOINT, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ action, proposal }) });
    setProposalResult(await response.json());
  };

  useEffect(() => {
    let cancelled = false;
    BundleEmbedder.init({
      manifestPath: MANIFEST_PATH,
      loader: new FetchBundleLoader(),
      platform: "web",
    })
      .then((instance) => {
        if (cancelled) {
          return;
        }
        instance.subscribe((event) => {
          setEvents((previous) => [...previous, event]);
        });
        setEmbedder(instance);
        setEvidence(instance.releaseEvidence());
      })
      .catch((error) => {
        if (!cancelled) {
          setInitError(String(error));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const submit = () => {
    if (embedder === null) {
      return;
    }
    embedder.submit(CAPABILITY_ID, { note });
  };

  return h(
    "div",
    null,
    h("h1", null, "traverse-embedder-web — traverse-starter, no sidecar"),
    h(
      "p",
      null,
      "This page loads the checked-in traverse-starter application bundle directly " +
        "from the repository over static HTTP and executes its bundled WASM capability " +
        "in this browser tab via BundleEmbedder — there is no traverse-cli serve process " +
        "involved.",
    ),
    h("div", { className: "card" },
      h("h2", null, "Governed multi-capability proposal"),
      h("p", null, "This page does not host a planner. Review every node, edge, and mapping below; mappings remain unconfirmed until the runtime validates them. Submission never executes automatically."),
      h("textarea", { value: proposalText, onChange: (event) => setProposalText(event.target.value), rows: 14 }),
      h("p", null, "Candidate ambiguity is a validation outcome: choose or correct a proposal explicitly; this UI never auto-selects a catalog candidate."),
      h("button", { onClick: () => reviewProposal("validate") }, "Validate and review"), " ",
      h("button", { onClick: () => reviewProposal("submit") }, "Submit for authorization"), " ",
      h("button", { onClick: () => reviewProposal("execute") }, "Execute only if authorized"),
      proposalResult !== null && h("pre", null, JSON.stringify(proposalResult, null, 2)),
    ),
    initError !== null &&
      h("div", { className: "card status-error" }, h("strong", null, "init failed: "), initError),
    embedder !== null &&
      h(
        "div",
        { className: "card" },
        h("p", { className: "status-ok" }, "Bundle initialized without a sidecar."),
        h("input", {
          value: note,
          onChange: (event) => setNote(event.target.value),
        }),
        h("button", { onClick: submit }, "Submit " + CAPABILITY_ID),
      ),
    h(
      "div",
      { className: "card" },
      h("h2", null, "Events"),
      eventList(events),
    ),
    evidence !== null &&
      h(
        "div",
        { className: "card" },
        h("h2", null, "Release evidence"),
        h("pre", null, JSON.stringify(evidence, null, 2)),
      ),
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(h(App));
