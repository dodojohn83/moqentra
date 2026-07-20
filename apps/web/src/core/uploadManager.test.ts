import { describe, expect, it } from "vitest";
import { UploadManager } from "./uploadManager";

describe("UploadManager", () => {
  it("uploads file in chunks and tracks progress", async () => {
    const manager = new UploadManager();
    const file = new File(["x".repeat(6 * 1024 * 1024)], "test.bin");
    const chunkFn = async (chunk: Blob, index: number) => ({ etag: `etag-${index}`, chunkIndex: index });

    const id = manager.start(file, chunkFn);
    await new Promise((r) => setTimeout(r, 50));

    const state = manager.getState(id);
    expect(state).toBeDefined();
    expect(state!.completed).toBe(true);
    expect(state!.progress).toBe(100);
  });

  it("cancels an upload", async () => {
    const manager = new UploadManager();
    const file = new File(["data"], "test.txt");
    let called = false;
    const chunkFn = async (_chunk: Blob, _index: number, signal: AbortSignal) => {
      called = true;
      if (signal.aborted) throw new Error("aborted");
      return { etag: "e", chunkIndex: 0 };
    };

    const id = manager.start(file, chunkFn);
    manager.cancel(id);
    await new Promise((r) => setTimeout(r, 30));
    expect(called).toBe(true);
    const state = manager.getState(id);
    expect(state?.abortController.signal.aborted).toBe(true);
  });
});
