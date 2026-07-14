# Dashboard Gateway Audit

Date: 2026-07-09

## Context

The dashboard can surface low-level transport failures directly to operators: unexpected empty JSON, stream proxy timeouts, and generic bad-gateway messages. These are not only stream problems. They show the dashboard gateway does not normalize backend failures before the UI consumes them.

This audit covers Candidate 5 from the architecture review: Dashboard Gateway.

## Current Shape

`cli/src/native/stream/dashboard.rs` owns a broad gateway surface:

- static dashboard asset serving
- `/api/service/*` proxying
- service request focus proxying
- CLI fallback for some service APIs
- local loopback proxying
- dashboard service-status response repair
- stream API routing
- session screenshot and tab endpoints
- JSON response writing

The local proxy path is low-level:

- `proxy_local_http_api_request` connects to `127.0.0.1:<port>`
- `proxy_local_http_api_request_with_timeout` writes raw HTTP bytes
- `read_local_http_response` reads bytes until EOF or content length
- callers receive `Vec<u8>` or a string error

The UI then sees failures through uneven shapes: sometimes a proxied backend response, sometimes a CLI fallback response, sometimes `{ success: false, error }`, and sometimes an empty or invalid body that causes a client parse error.

## Failure Mechanism

The gateway currently has no typed failure interface between local backend transport and dashboard HTTP responses. That allows these failure modes:

- timeout text leaks as the primary operator message
- empty backend response can become a client JSON parse failure
- non-2xx backend responses are sometimes fallback candidates and sometimes raw responses
- service status gets special repair, while screenshots and streams do not share the same normalization
- callers cannot distinguish connect timeout, read timeout, invalid payload, backend error, and unavailable service in a stable way

This makes UX depend on which branch of the broad handler served a request.

## Deletion Test

Deleting `proxy_local_http_api_request` would remove one transport helper, but every caller would still need to decide how to convert backend transport into dashboard HTTP.

Deleting `handle_service_api_request` would remove service proxying, but the broad dashboard handler would still mix static assets, stream APIs, and service APIs.

Deleting `repair_dashboard_service_status_response` would make status less compatible, but would not create a general gateway contract.

## Recommended Deep Module

Create a Dashboard Gateway module with a typed interface:

```text
DashboardGatewayRequest
DashboardGatewayResponse
DashboardGatewayError
```

The gateway should normalize local backend transport and backend payload failures into stable JSON envelopes before they reach dashboard code.

## Required Interlocks

- Empty backend response must become a structured JSON error with a stable code.
- Connect, write, and read timeouts must become distinct stable codes.
- Backend non-JSON where JSON is expected must become `invalid_backend_payload`.
- Service API fallback should be explicit in the response metadata.
- Every dashboard API path should return a valid HTTP response body, even on proxy failure.

## Risks

- A large rewrite of `handle_dashboard_connection` would risk dashboard availability.
- A gateway module that only handles `/api/service/*` would leave screenshot and stream paths brittle.
- Adding stable codes without tests would still allow regressions in response shape.

## Acceptance For Candidate 5

- A plan exists to extract typed gateway response/error handling.
- The first implementation slice should normalize empty response, timeout, and invalid payload errors.
- Focused tests should cover local proxy response parsing and JSON error envelope generation.
- The broad handler can adopt the typed gateway incrementally.
