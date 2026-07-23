import { describe, expect, it, beforeEach } from "vitest";
import {
  __resetCacheForTests,
  clearTenant,
  getCached,
  makeCacheKey,
  setCached,
} from "./queryCache";

describe("queryCache", () => {
  beforeEach(() => __resetCacheForTests());

  it("scopes keys by tenant and project", () => {
    const a = makeCacheKey({
      tenantId: "t1",
      projectId: "p1",
      resource: "datasets",
      id: "d1",
    });
    const b = makeCacheKey({
      tenantId: "t2",
      projectId: "p1",
      resource: "datasets",
      id: "d1",
    });
    expect(a).not.toEqual(b);
    setCached(a, { name: "ds" });
    expect(getCached(a)).toEqual({ name: "ds" });
    expect(getCached(b)).toBeUndefined();
  });

  it("clears previous tenant on switch", () => {
    const key = makeCacheKey({
      tenantId: "t1",
      resource: "models",
    });
    setCached(key, 1);
    clearTenant("t1");
    expect(getCached(key)).toBeUndefined();
  });
});
