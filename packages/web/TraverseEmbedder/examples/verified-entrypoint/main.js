import { executeVerifiedEntrypoint, VerifiedEntrypointError } from "/pkg/verifiedEntrypoint.js";

const form = document.querySelector("#execute-form");
const result = document.querySelector("#result");
const capabilityId = document.querySelector("#capability-id");
const capabilityVersion = document.querySelector("#capability-version");
const serverUrl = document.querySelector("#server-url");
const runtimeRequest = document.querySelector("#runtime-request");

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  try {
    const request = JSON.parse(runtimeRequest.value);
    const response = await executeVerifiedEntrypoint(fetch, serverUrl.value, {
      id: capabilityId.value,
      version: capabilityVersion.value,
      request,
    });
    result.textContent = JSON.stringify(response, null, 2);
  } catch (error) {
    const safe = error instanceof VerifiedEntrypointError
      ? { code: error.code, message: error.message }
      : { code: "invalid_browser_request", message: "RuntimeRequest must be valid JSON." };
    result.textContent = JSON.stringify({ status: "error", error: safe }, null, 2);
  }
});
