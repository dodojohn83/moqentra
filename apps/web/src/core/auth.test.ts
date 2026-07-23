import { describe, expect, it } from "vitest";
import { tokenManager } from "./auth";

describe("tokenManager", () => {
  it("keeps access token only in memory", () => {
    tokenManager.clear();
    expect(tokenManager.getAccessToken()).toBeUndefined();
    tokenManager.setAccessToken("test-token");
    expect(tokenManager.getAccessToken()).toBe("test-token");
    tokenManager.clear();
    expect(tokenManager.getAccessToken()).toBeUndefined();
  });
});
