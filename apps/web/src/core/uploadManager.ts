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

const PROGRESS_KEY = "moqentra.upload.progress";

export interface PersistedUploadProgress {
  id: string;
  fileName: string;
  fileSize: number;
  completedChunks: number;
  etags: string[];
  progress: number;
  sessionId?: string;
}

function loadProgressMap(): Record<string, PersistedUploadProgress> {
  try {
    const raw = sessionStorage.getItem(PROGRESS_KEY);
    return raw ? (JSON.parse(raw) as Record<string, PersistedUploadProgress>) : {};
  } catch {
    return {};
  }
}

function saveProgressMap(map: Record<string, PersistedUploadProgress>): void {
  try {
    sessionStorage.setItem(PROGRESS_KEY, JSON.stringify(map));
  } catch {
    /* ignore quota */
  }
}

export class UploadManager {
  private uploads = new Map<string, UploadState>();

  /** List server-recoverable upload progress (after refresh). */
  listPersisted(): PersistedUploadProgress[] {
    return Object.values(loadProgressMap());
  }

  clearPersisted(id: string): void {
    const map = loadProgressMap();
    delete map[id];
    saveProgressMap(map);
  }

  private persistProgress(id: string, state: UploadState, sessionId?: string): void {
    const map = loadProgressMap();
    map[id] = {
      id,
      fileName: state.file.name,
      fileSize: state.file.size,
      completedChunks: state.completedChunks,
      etags: state.etags,
      progress: state.progress,
      sessionId,
    };
    saveProgressMap(map);
  }

  start(
    file: File,
    uploadChunk: (chunk: Blob, index: number, signal: AbortSignal) => Promise<ChunkResult>,
    opts?: { id?: string; completedChunks?: number; etags?: string[]; sessionId?: string },
  ): string {
    const id =
      opts?.id ??
      (typeof crypto !== "undefined" && crypto.randomUUID
        ? crypto.randomUUID()
        : `upload-${Date.now()}-${Math.random().toString(36).slice(2)}`);
    const abortController = new AbortController();
    const state: UploadState = {
      file,
      progress: opts?.completedChunks
        ? Math.round(((opts.completedChunks) / Math.max(1, Math.ceil(file.size / CHUNK_SIZE))) * 100)
        : 0,
      completed: false,
      uploading: false,
      abortController,
      completedChunks: opts?.completedChunks ?? 0,
      etags: opts?.etags ?? [],
    };
    this.uploads.set(id, state);
    this.persistProgress(id, state, opts?.sessionId);

    this.run(id, uploadChunk, opts?.sessionId);
    return id;
  }

  private run(
    id: string,
    uploadChunk: (chunk: Blob, index: number, signal: AbortSignal) => Promise<ChunkResult>,
    sessionId?: string,
  ): void {
    const state = this.uploads.get(id);
    if (!state || state.completed || state.uploading) return;

    const totalChunks = Math.ceil(state.file.size / CHUNK_SIZE);
    if (totalChunks === 0) {
      state.progress = 100;
      state.completed = true;
      this.clearPersisted(id);
      return;
    }

    state.uploading = true;

    (async () => {
      try {
        for (let i = state.completedChunks; i < totalChunks; i++) {
          if (state.abortController.signal.aborted) {
            this.persistProgress(id, state, sessionId);
            return;
          }
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
          this.persistProgress(id, state, sessionId);
        }
        state.completed = true;
        this.clearPersisted(id);
      } catch (e) {
        state.error = e instanceof Error ? e.message : String(e);
        this.persistProgress(id, state, sessionId);
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
      this.persistProgress(id, state);
      return true;
    }
    return false;
  }

  resume(
    id: string,
    uploadChunk: (chunk: Blob, index: number, signal: AbortSignal) => Promise<ChunkResult>,
  ): boolean {
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

