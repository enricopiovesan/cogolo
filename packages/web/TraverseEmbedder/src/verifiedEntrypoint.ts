/**
 * Browser client for the Spec 115 verified-entrypoint HTTP boundary.
 *
 * The server owns registry synchronization, artifact materialization, digest
 * verification, and execution. This client deliberately sends only an exact
 * identity and a RuntimeRequest; it never accepts an artifact URL or bytes.
 */

export interface VerifiedEntrypointRequest {
  readonly id: string;
  readonly version: string;
  readonly request: Record<string, unknown>;
}

export interface VerifiedEntrypointResponse {
  readonly status: "completed" | "error";
  readonly request_id: string;
  readonly execution_id: string;
  readonly trace_ref: string;
  readonly output: unknown;
  readonly error: { readonly code: string; readonly message: string } | null;
  readonly trace: unknown;
}

export class VerifiedEntrypointError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "VerifiedEntrypointError";
    this.code = code;
  }
}

export type VerifiedEntrypointFetch = (
  input: string,
  init: RequestInit,
) => Promise<Response>;

function exact(value: string, label: string): string {
  const normalized = value.trim();
  if (normalized.length === 0) {
    throw new VerifiedEntrypointError("invalid_entrypoint_request", `${label} is required`);
  }
  return normalized;
}

function endpointFor(serverUrl: string): string {
  return `${exact(serverUrl, "server URL").replace(/\/+$/, "")}/v1/entrypoints/execute`;
}

/** Execute one capability whose artifact was verified by the serving host. */
export async function executeVerifiedEntrypoint(
  fetcher: VerifiedEntrypointFetch,
  serverUrl: string,
  request: VerifiedEntrypointRequest,
): Promise<VerifiedEntrypointResponse> {
  const id = exact(request.id, "capability id");
  const version = exact(request.version, "capability version");
  const response = await fetcher(endpointFor(serverUrl), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ entrypoint_kind: "capability", id, version, request: request.request }),
  });
  const body: unknown = await response.json();
  if (!response.ok) {
    const problem = body as { traverse_code?: unknown; detail?: unknown };
    throw new VerifiedEntrypointError(
      typeof problem.traverse_code === "string" ? problem.traverse_code : "entrypoint_execute_failed",
      typeof problem.detail === "string" ? problem.detail : `verified entrypoint request failed: HTTP ${response.status}`,
    );
  }
  return body as VerifiedEntrypointResponse;
}
