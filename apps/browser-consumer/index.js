(function (root, factory) {
  const client = factory();

  if (typeof module === "object" && module.exports) {
    module.exports = client;
  }

  if (root) {
    root.TraverseBrowserConsumer = client;
  }
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  let baseClient = null;

  if (typeof require === "function") {
    try {
      baseClient = require("../react-demo/src/browser-adapter-client.js");
    } catch {
      baseClient = null;
    }
  }

  if (!baseClient && typeof globalThis !== "undefined" && globalThis.TraverseReactDemoClient) {
    baseClient = globalThis.TraverseReactDemoClient;
  }

  if (!baseClient) {
    throw new Error(
      "Traverse browser consumer requires the approved browser adapter client to be available.",
    );
  }

  const APPROVED_BROWSER_CONSUMER_SESSION = {
    ...baseClient.APPROVED_BROWSER_DEMO_SESSION,
    title: "Traverse Browser Consumer",
    summary:
      "Traverse's browser-targeted consumer facade for downstream browser-hosted apps like youaskm3.",
  };

  function createBrowserConsumerState() {
    return baseClient.createLiveDemoState();
  }

  function buildBrowserConsumerSubscriptionRequest() {
    return baseClient.buildApprovedSubscriptionRequest();
  }

  function runBrowserConsumerSubscription(options = {}) {
    return baseClient.runLiveBrowserSubscription(options);
  }

  function applyBrowserConsumerMessage(state, message, created) {
    return baseClient.applyBrowserSubscriptionMessage(state, message, created);
  }

  function browserConsumerTraceSummary(trace, terminalResult) {
    return baseClient.traceSummary(trace, terminalResult);
  }

  return {
    APPROVED_BROWSER_CONSUMER_SESSION,
    applyBrowserConsumerMessage,
    browserConsumerTraceSummary,
    buildBrowserConsumerSubscriptionRequest,
    createBrowserConsumerState,
    runBrowserConsumerSubscription,
  };
});
