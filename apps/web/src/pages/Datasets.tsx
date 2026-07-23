import { FormEvent, useCallback, useEffect, useState } from "react";
import { useApi } from "../core/useApi";
import { useTenant } from "../core/TenantContext";
import {
  clearTenant,
  getCached,
  makeCacheKey,
  setCached,
} from "../core/queryCache";
import { VirtualList } from "../core/VirtualList";

interface DatasetRow {
  id?: string;
  name?: string;
  state?: string;
}

export function Datasets() {
  const api = useApi();
  const { scope, setTenant, setProject } = useTenant();
  const [items, setItems] = useState<DatasetRow[]>([]);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    if (!scope.tenantId) return;
    const key = makeCacheKey({
      tenantId: scope.tenantId,
      projectId: scope.projectId,
      resource: "datasets",
    });
    const cached = getCached<DatasetRow[]>(key);
    if (cached) setItems(cached);
    setBusy(true);
    setError(null);
    try {
      const page = await api.listDatasets({ xTenantId: scope.tenantId });
      const rows = (page.items ?? []) as DatasetRow[];
      setItems(rows);
      setCached(key, rows);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load datasets");
    } finally {
      setBusy(false);
    }
  }, [api, scope.projectId, scope.tenantId]);

  useEffect(() => {
    void load();
  }, [load]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!name.trim() || !scope.projectId || !scope.tenantId) {
      setError("Tenant, project and name are required");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await api.createDataset({
        xTenantId: scope.tenantId,
        createDatasetRequest: {
          name: name.trim(),
          project_id: scope.projectId,
        },
      });
      setName("");
      clearTenant(scope.tenantId); // force list refresh without stale cache
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Create failed");
    } finally {
      setBusy(false);
    }
  }

  function onTenantChange(tenantId: string) {
    clearTenant(scope.tenantId);
    setTenant(tenantId);
  }

  return (
    <section className="page">
      <h1>Datasets</h1>
      <p>
        Tenant{" "}
        <input
          aria-label="Tenant ID"
          value={scope.tenantId}
          onChange={(e) => onTenantChange(e.target.value)}
        />{" "}
        Project{" "}
        <input
          aria-label="Project ID"
          value={scope.projectId ?? ""}
          onChange={(e) => setProject(e.target.value || undefined)}
        />
      </p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <form onSubmit={onCreate}>
        <label>
          Name
          <input value={name} onChange={(e) => setName(e.target.value)} required />
        </label>
        <button type="submit" disabled={busy}>
          Create dataset
        </button>
      </form>
      {busy && <p>Loading…</p>}
      {items.length > 20 ? (
        <VirtualList
          items={items}
          rowHeight={36}
          height={360}
          renderRow={(d) => (
            <span>
              <strong>{d.name}</strong> — {d.state} <code>{d.id}</code>
            </span>
          )}
        />
      ) : (
        <ul>
          {items.map((d) => (
            <li key={d.id ?? d.name}>
              <strong>{d.name}</strong> — {d.state} <code>{d.id}</code>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
