export interface UploadState {
  file: File;
  progress: number;
  completed: boolean;
  uploading: boolean;
  error?: string;
  abortController: AbortController;
  completedChunks: number;
  etags: string[];
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
      uploading: false,
      abortController,
      completedChunks: 0,
      etags: [],
    };
    this.uploads.set(id, state);

    this.run(id, uploadChunk);
    return id;
  }

  private run(
    id: string,
    uploadChunk: (chunk: Blob, index: number, signal: AbortSignal) => Promise<ChunkResult>,
  ): void {
    const state = this.uploads.get(id);
    if (!state || state.completed || state.uploading) return;

    const totalChunks = Math.ceil(state.file.size / CHUNK_SIZE);
    if (totalChunks === 0) {
      state.progress = 100;
      state.completed = true;
      return;
    }

    state.uploading = true;

    (async () => {
      try {
        for (let i = state.completedChunks; i < totalChunks; i++) {
          if (state.abortController.signal.aborted) return;
          const chunk = state.file.slice(
            i * CHUNK_SIZE,
            Math.min((i + 1) * CHUNK_SIZE, state.file.size),
          );
          const result = await uploadChunk(chunk, i, state.abortController.signal);
          if (result.chunkIndex < 0 || result.chunkIndex >= totalChunks || result.chunkIndex !== i) {
            throw new Error(`chunk index mismatch: expected ${i}, got ${result.chunkIndex}`);
          }
          if (!result.etag) {
            throw new Error(`missing etag for chunk ${i}`);
          }
          state.etags[result.chunkIndex] = result.etag;
          state.completedChunks = i + 1;
          state.progress = Math.round(((i + 1) / totalChunks) * 100);
        }
        state.completed = true;
      } catch (e) {
        state.error = e instanceof Error ? e.message : String(e);
      } finally {
        state.uploading = false;
      }
    })();
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
    if (!state || state.completed || state.uploading) return false;
    if (state.error) {
      state.error = undefined;
    }
    if (state.abortController.signal.aborted) {
      state.abortController = new AbortController();
    }
    this.run(id, uploadChunk);
    return true;
  }
}
