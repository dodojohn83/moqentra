/**
 * SSE client with persistent cursor, reconnect, and event dedup (R1-WEB-005).
 */

export type SseHandler = (event: MessageEvent, cursor: string | null) => void;

const CURSOR_KEY = "moqentra.sse.cursor";

export function loadSseCursor(): string | null {
  try {
    return sessionStorage.getItem(CURSOR_KEY);
  } catch {
    return null;
  }
}

export function saveSseCursor(cursor: string | null): void {
  try {
    if (cursor) sessionStorage.setItem(CURSOR_KEY, cursor);
    else sessionStorage.removeItem(CURSOR_KEY);
  } catch {
    /* ignore */
  }
}

export class SseClient {
  private es: EventSource | null = null;
  private seen = new Set<string>();
  private closed = false;
  private retryMs = 1000;
  private readonly maxRetryMs = 30_000;

  constructor(
    private readonly urlBuilder: (cursor: string | null) => string,
    private readonly onEvent: SseHandler,
    private readonly onNeedResync?: () => void,
  ) {}

  start(): void {
    this.closed = false;
    this.connect(loadSseCursor());
  }

  stop(): void {
    this.closed = true;
    this.es?.close();
    this.es = null;
  }

  private connect(cursor: string | null): void {
    if (this.closed) return;
    const url = this.urlBuilder(cursor);
    this.es?.close();
    const es = new EventSource(url, { withCredentials: true });
    this.es = es;

    es.onmessage = (ev) => {
      const id = ev.lastEventId || `${ev.data}`;
      if (this.seen.has(id)) return;
      this.seen.add(id);
      if (this.seen.size > 5000) {
        // bound memory: drop oldest half
        this.seen = new Set([...this.seen].slice(-2500));
      }
      const nextCursor = ev.lastEventId || cursor;
      if (nextCursor) saveSseCursor(nextCursor);
      this.onEvent(ev, nextCursor);
      this.retryMs = 1000;
    };

    es.onerror = () => {
      es.close();
      this.es = null;
      // Cursor invalid / stream lost: optional full resync
      if (this.retryMs >= this.maxRetryMs / 2) {
        this.onNeedResync?.();
      }
      const delay = this.retryMs;
      this.retryMs = Math.min(this.retryMs * 2, this.maxRetryMs);
      window.setTimeout(() => this.connect(loadSseCursor()), delay);
    };
  }
}
