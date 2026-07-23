import { useCallback, useEffect, useState } from "react";
import { LabelUAnnotator } from "../annotation/LabelUAnnotator";
import { useApi } from "../core/useApi";
import { useTenant } from "../core/TenantContext";
import { tokenManager } from "../core/auth";

interface ProjectRow {
  id?: string;
  name?: string;
  state?: string;
}

export function Annotations() {
  const api = useApi();
  const { scope } = useTenant();
  const [projects, setProjects] = useState<ProjectRow[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [mediaUrl, setMediaUrl] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!scope.tenantId) return;
    try {
      // Prefer generated client when list endpoint exists; fall back to fetch.
      const base = import.meta.env.VITE_API_BASE_URL || "";
      const token = tokenManager.getAccessToken();
      const res = await fetch(`${base}/v1/annotation-projects`, {
        headers: {
          Accept: "application/json",
          "X-Tenant-Id": scope.tenantId,
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const body = await res.json();
      const items = (body.items ?? body) as ProjectRow[];
      setProjects(Array.isArray(items) ? items : []);
      void api;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load annotation projects");
    }
  }, [api, scope.tenantId]);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <section className="page">
      <h1>Annotation</h1>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <ul>
        {projects.map((p) => (
          <li key={p.id}>
            <button type="button" onClick={() => setSelected(p.id ?? null)}>
              {p.name} ({p.state})
            </button>
          </li>
        ))}
      </ul>
      {selected && (
        <div>
          <h2>LabelU task workspace</h2>
          <label>
            Media URL
            <input
              value={mediaUrl}
              onChange={(e) => setMediaUrl(e.target.value)}
              placeholder="Signed media URL"
            />
          </label>
          {mediaUrl ? (
            <LabelUAnnotator mediaUrl={mediaUrl} sampleName={selected} />
          ) : (
            <p>Provide a task media URL after claim to open the annotator.</p>
          )}
        </div>
      )}
    </section>
  );
}
