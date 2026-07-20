import { describe, expect, it, vi } from "vitest";
import { ApiError, apiRequest } from "./apiClient";

describe("apiClient", () => {
  it("sends idempotency and etag headers", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      text: async () => "{}",
    });
    vi.stubGlobal("fetch", fetchMock);

    await apiRequest(
      "https://api.example.com",
      {
        method: "POST",
        path: "/datasets",
        body: { name: "ds" },
        idempotencyKey: "key-1",
        ifMatch: '"abc"',
      },
      "token",
    );

    const [, init] = fetchMock.mock.calls[0];
    const headers = init.headers as Record<string, string>;
    expect(headers["Authorization"]).toBe("Bearer token");
    expect(headers["Idempotency-Key"]).toBe("key-1");
    expect(headers["If-Match"]).toBe('"abc"');
  });

  it("parses problem details and strips secrets", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({
        status: 400,
        code: "INVALID_ARGUMENT",
        detail: "bad token=secret",
      }),
    });
    vi.stubGlobal("fetch", fetchMock);

    await expect(
      apiRequest("https://api.example.com", { method: "GET", path: "/x" }),
    ).rejects.toBeInstanceOf(ApiError);
  });
});
