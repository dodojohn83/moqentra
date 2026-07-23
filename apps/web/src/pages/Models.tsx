import { FormEvent, useCallback, useEffect, useState } from "react";
import { useApi } from "../core/useApi";
import { useTenant } from "../core/TenantContext";

interface ModelRow {
  id?: string;
  name?: string;
}

export function Models() {
  const api = useApi();
  const { scope } = useTenant();
  const [items, setItems] = useState<ModelRow[]>([]);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!scope.tenantId) return;
    try {
      const page = await api.listModels({ xTenantId: scope.tenantId });
      setItems((page.items ?? []) as ModelRow[]);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load models");
    }
  }, [api, scope.tenantId]);

  useEffect(() => {
    void load();
  }, [load]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!scope.projectId || !scope.tenantId) {
      setError("Project required");
      return;
    }
    try {
      await api.createModel({
        xTenantId: scope.tenantId,
        createModelRequest: { name, project_id: scope.projectId },
      });
      setName("");
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Create failed");
    }
  }

  return (
    <section className="page">
      <h1>Models</h1>
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
        <button type="submit">Register model</button>
      </form>
      <ul>
        {items.map((m) => (
          <li key={m.id}>
            {m.name} <code>{m.id}</code>
          </li>
        ))}
      </ul>
    </section>
  );
}
