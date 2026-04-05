# Data Model: Local Browser Adapter Transport

## Purpose

This document defines the implementation-tight transport artifacts for the `019-local-browser-adapter-transport` slice.

It governs how one local browser adapter creates and exposes one browser subscription stream while reusing the payload artifacts from `013-browser-runtime-subscription`.

## 1. Create-Subscription HTTP Request

Represents one local HTTP request that creates one browser-consumable subscription stream.

### HTTP Shape

- Method: `POST`
- Path: `/local/browser-subscriptions`
- Content-Type: `application/json`

### Body Shape

The body embeds the already-governed browser subscription request artifact.

```json
{
  "subscription_request": {
    "kind": "browser_runtime_subscription_request",
    "schema_version": "1.0.0",
    "governing_spec": "013-browser-runtime-subscription",
    "request_id": "req_20260402_0001"
  }
}
```

### Rules

- `subscription_request` MUST conform to `013-browser-runtime-subscription`.
- The adapter MUST NOT define alternate selectors or adapter-only target semantics in this slice.

## 2. Create-Subscription Success Response

Represents one successful local subscription setup response.

### HTTP Shape

- Status: `201 Created`
- Content-Type: `application/json`

### Required Fields

- `kind`
- `schema_version`
- `governing_spec`
- `subscription_id`
- `stream_url`
- `request_id`
- `execution_id`

### Shape

```json
{
  "kind": "local_browser_subscription_created",
  "schema_version": "1.0.0",
  "governing_spec": "019-local-browser-adapter-transport",
  "subscription_id": "lbs_20260402_0001",
  "stream_url": "/local/browser-subscriptions/lbs_20260402_0001/stream",
  "request_id": "req_20260402_0001",
  "execution_id": "exec_20260402_0001"
}
```

### Rules

- `stream_url` MUST identify exactly one created local stream.
- The create response MUST be returned only after the adapter has validated the target and established the identifiers needed for the stream.
- The adapter MAY normalize from `request_id` to `execution_id` during setup, but it MUST return both resolved identifiers in the success response.

## 3. Create-Subscription Setup Error Response

Represents one machine-readable failure during local subscription setup.

### HTTP Shape

- Status: `400 Bad Request` for invalid input
- Status: `404 Not Found` for missing runtime outcomes
- Content-Type: `application/json`

### Required Fields

- `kind`
- `schema_version`
- `governing_spec`
- `code`
- `message`

### Error Code Enum

- `invalid_request`
- `not_found`

### Shape

```json
{
  "kind": "local_browser_subscription_setup_error",
  "schema_version": "1.0.0",
  "governing_spec": "019-local-browser-adapter-transport",
  "code": "not_found",
  "message": "no runtime outcome matched request_id=req_20260402_0001"
}
```

### Rules

- Setup errors occur before stream creation.
- Setup errors MUST NOT return a `stream_url`.
- The adapter SHOULD reuse the same error-code meanings already governed for browser subscription validation when possible.

## 4. Stream Retrieval HTTP Request

Represents one browser request for an already-created local stream.

### HTTP Shape

- Method: `GET`
- Path: `/local/browser-subscriptions/{subscription_id}/stream`
- Accept: `text/event-stream`

### Rules

- `{subscription_id}` MUST refer to a previously created local browser subscription.
- The adapter MUST target exactly one stream per `subscription_id`.

## 5. Stream Retrieval Not Found Error

Represents one adapter-level failure when the requested stream does not exist.

### HTTP Shape

- Status: `404 Not Found`
- Content-Type: `application/json`

### Shape

```json
{
  "kind": "local_browser_subscription_stream_error",
  "schema_version": "1.0.0",
  "governing_spec": "019-local-browser-adapter-transport",
  "code": "not_found",
  "message": "subscription_id lbs_20260402_0001 was not found"
}
```

### Rules

- This error is adapter-scoped and refers to missing created streams, not missing runtime outcome selectors during setup.

## 6. Server-Sent Events Frame

Represents one SSE frame carrying one approved browser subscription message.

### HTTP Shape

- Status: `200 OK`
- Content-Type: `text/event-stream`

### Frame Shape

```text
event: traverse_message
data: {"kind":"browser_runtime_subscription_lifecycle","schema_version":"1.0.0","sequence":0,"request_id":"req_20260402_0001","execution_id":"exec_20260402_0001","status":"subscription_established"}

```

### Rules

- The `data` payload MUST be one valid `013-browser-runtime-subscription` message artifact.
- The adapter MUST preserve message ordering and sequence values from the governed browser subscription stream.
- The adapter MUST NOT rename governed message kinds into adapter-specific payload kinds.

## 7. Valid Stream Lifecycle

Represents the complete local adapter flow.

### Ordered Flow

1. Client `POST`s one create-subscription request.
2. Adapter validates the embedded `013` request and resolves the targeted runtime outcome.
3. Adapter returns `201 Created` with `subscription_id` and `stream_url`, or a deterministic setup error.
4. Client `GET`s the `stream_url` with `Accept: text/event-stream`.
5. Adapter emits the governed browser subscription messages over SSE in the approved order.
6. Stream closes after the governed `stream_completed` message.

### Rules

- The create phase and stream phase are separate.
- Setup failures happen in the create phase.
- Stream-not-found failures happen in the GET phase.

## 8. Relationship To Other Slices

- `013-browser-runtime-subscription` governs the subscription request payload and streamed message artifacts.
- `019-local-browser-adapter-transport` governs HTTP paths, setup responses, stream framing, and adapter-level not-found behavior.
- Future auth, replay, multiplexing, or remote exposure slices must extend this transport or replace it explicitly; they must not silently mutate the governed `013` payload contract.
