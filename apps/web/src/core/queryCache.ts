/**
 * Tenant/project-scoped query cache (R1-WEB-003).
 * Keys include tenant, project, resource and optional revision.
 */

export type CacheKey = string;

export interface CacheEntry<T> {
  value: T;
  updatedAt: number;
  abort?: AbortController;
}

const store = new Map<CacheKey, CacheEntry<unknown>>();
const inflight = new Map<CacheKey, AbortController>();

export function makeCacheKey(parts: {
  tenantId: string;
  projectId?: string;
  resource: string;
  id?: string;
  revision?: string | number;
}): CacheKey {
  return [
    parts.tenantId,
    parts.projectId ?? "-",
    parts.resource,
    parts.id ?? "-",
    parts.revision ?? "-",
  ].join("|");
}

export function getCached<T>(key: CacheKey): T | undefined {
  return store.get(key)?.value as T | undefined;
}

export function setCached<T>(key: CacheKey, value: T): void {
  store.set(key, { value, updatedAt: Date.now() });
}

export function invalidatePrefix(prefix: string): void {
  for (const key of [...store.keys()]) {
    if (key.startsWith(prefix)) store.delete(key);
  }
  for (const [key, controller] of [...inflight.entries()]) {
    if (key.startsWith(prefix)) {
      controller.abort();
      inflight.delete(key);
    }
  }
}

/** Call when switching tenant: cancel uploads/SSE callers should also abort. */
export function clearTenant(tenantId: string): void {
  invalidatePrefix(`${tenantId}|`);
}

export function trackInflight(key: CacheKey, controller: AbortController): void {
  inflight.get(key)?.abort();
  inflight.set(key, controller);
}

export function clearInflight(key: CacheKey): void {
  inflight.delete(key);
}

export function __resetCacheForTests(): void {
  store.clear();
  for (const c of inflight.values()) c.abort();
  inflight.clear();
}
