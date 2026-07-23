/**
 * Persist Operation progress so refresh recovers server-side status (R1-WEB-016).
 */

export interface TrackedOperation {
  id: string;
  statusUrl: string;
  kind: string;
  createdAt: number;
  cancelled?: boolean;
}

const KEY = "moqentra.operations";

function load(): TrackedOperation[] {
  try {
    const raw = sessionStorage.getItem(KEY);
    return raw ? (JSON.parse(raw) as TrackedOperation[]) : [];
  } catch {
    return [];
  }
}

function save(ops: TrackedOperation[]): void {
  try {
    sessionStorage.setItem(KEY, JSON.stringify(ops.slice(-50)));
  } catch {
    /* ignore */
  }
}

export function trackOperation(op: TrackedOperation): void {
  const ops = load().filter((o) => o.id !== op.id);
  ops.push(op);
  save(ops);
}

export function listOperations(): TrackedOperation[] {
  return load();
}

export function markCancelled(id: string): void {
  const ops = load().map((o) => (o.id === id ? { ...o, cancelled: true } : o));
  save(ops);
}

export function removeOperation(id: string): void {
  save(load().filter((o) => o.id !== id));
}

/** Poll status URLs for non-cancelled operations; caller supplies fetch. */
export async function recoverOperations(
  fetchStatus: (statusUrl: string, signal: AbortSignal) => Promise<{ done: boolean; status: string }>,
): Promise<Array<TrackedOperation & { status: string }>> {
  const out: Array<TrackedOperation & { status: string }> = [];
  for (const op of load()) {
    if (op.cancelled) {
      out.push({ ...op, status: "cancelled" });
      continue;
    }
    const ac = new AbortController();
    try {
      const res = await fetchStatus(op.statusUrl, ac.signal);
      out.push({ ...op, status: res.status });
      if (res.done) removeOperation(op.id);
    } catch {
      out.push({ ...op, status: "unknown" });
    }
  }
  return out;
}
