export interface UploadState {
  file: File;
  progress: number;
  completed: boolean;
  error?: string;
  abortController: AbortController;
}

export interface ChunkResult {
  etag: string;
  chunkIndex: number;
}

const CHUNK_SIZE = 5 * 1024 * 1024;

export class UploadManager {
  private uploads = new Map<string, UploadState>();

  start(file: File, uploadChunk: (chunk: Blob, index: number, signal: AbortSignal) => Promise<ChunkResult>): string {
    const id =
      typeof crypto !== "undefined" && crypto.randomUUID
        ? crypto.randomUUID()
        : `upload-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const abortController = new AbortController();
    const state: UploadState = {
      file,
      progress: 0,
      completed: false,
      abortController,
    };
    this.uploads.set(id, state);

    const totalChunks = Math.ceil(file.size / CHUNK_SIZE);
    const chunks: ChunkResult[] = [];

    (async () => {
      try {
        for (let i = 0; i < totalChunks; i++) {
          if (abortController.signal.aborted) return;
          const chunk = file.slice(i * CHUNK_SIZE, Math.min((i + 1) * CHUNK_SIZE, file.size));
          const result = await uploadChunk(chunk, i, abortController.signal);
          chunks.push(result);
          state.progress = Math.round(((i + 1) / totalChunks) * 100);
        }
        state.completed = true;
      } catch (e) {
        state.error = e instanceof Error ? e.message : String(e);
      }
    })();

    return id;
  }

  getState(id: string): UploadState | undefined {
    return this.uploads.get(id);
  }

  cancel(id: string): boolean {
    const state = this.uploads.get(id);
    if (state) {
      state.abortController.abort();
      return true;
    }
    return false;
  }

  resume(id: string, uploadChunk: (chunk: Blob, index: number, signal: AbortSignal) => Promise<ChunkResult>): boolean {
    const state = this.uploads.get(id);
    if (!state || state.completed || state.error) return false;
    // In a real implementation this would re-send only missing chunks using the
    // stored etags. The placeholder resumes by creating a fresh AbortController.
    state.abortController = new AbortController();
    return true;
  }
}
