import { stripSecrets } from "./security";

export interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  code: string;
  detail: string;
  request_id: string;
}

export interface ApiRequest {
  method: string;
  path: string;
  body?: unknown;
  idempotencyKey?: string;
  ifMatch?: string;
}

export class ApiError extends Error {
  constructor(
    public readonly problem: ProblemDetails,
    public readonly response: Response,
  ) {
    super(`${problem.code}: ${problem.detail}`);
  }
}

export async function apiRequest(
  baseUrl: string,
  req: ApiRequest,
  token?: string,
): Promise<unknown> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "application/json",
  };
  if (token) headers["Authorization"] = `Bearer ${token}`;
  if (req.idempotencyKey) headers["Idempotency-Key"] = req.idempotencyKey;
  if (req.ifMatch) headers["If-Match"] = req.ifMatch;

  const path = req.path.startsWith("/") ? req.path : `/${req.path}`;
  const url = `${baseUrl.replace(/\/$/, "")}${path}`;
  const response = await fetch(url, {
    method: req.method,
    headers,
    body: req.body ? JSON.stringify(req.body) : undefined,
  });

  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as Partial<ProblemDetails>;
    const problem: ProblemDetails = {
      type: body.type ?? "about:blank",
      title: body.title ?? response.statusText,
      status: body.status ?? response.status,
      code: body.code ?? `HTTP_${response.status}`,
      detail: body.detail ? stripSecrets(body.detail) : response.statusText,
      request_id: body.request_id ?? "",
    };
    throw new ApiError(problem, response);
  }

  if (response.status === 202) {
    const body = (await response.json().catch(() => ({}))) as { status_url?: string };
    return { operationUrl: body.status_url };
  }

  const text = await response.text();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    throw new ApiError(
      {
        type: "about:blank",
        title: "Invalid JSON",
        status: response.status,
        code: "INVALID_JSON",
        detail: "server returned non-JSON response body",
        request_id: headers["Idempotency-Key"] ?? "",
      },
      response,
    );
  }
}

export async function* apiStream<T>(
  baseUrl: string,
  path: string,
  token?: string,
): AsyncGenerator<T, void, unknown> {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const url = `${baseUrl.replace(/\/$/, "")}${normalizedPath}`;
  const response = await fetch(url, {
    headers: {
      Accept: "text/event-stream",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
  });
  if (!response.ok) throw new Error(`SSE failed: ${response.status}`);
  const reader = response.body?.getReader();
  if (!reader) throw new Error("SSE body unavailable");

  const decoder = new TextDecoder();
  let buffer = "";
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";
      for (const line of lines) {
        if (line.startsWith("data: ")) {
          const payload = line.slice(6);
          if (!payload) continue;
          try {
            yield JSON.parse(payload) as T;
          } catch {
            // Skip malformed SSE payloads instead of crashing the stream.
          }
        }
      }
    }
  } finally {
    reader.cancel().catch(() => {});
  }
}
