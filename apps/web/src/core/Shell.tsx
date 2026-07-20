import { useTenant } from "./TenantContext";

export function Shell() {
  const { scope, setTenant } = useTenant();
  return (
    <div>
      <header>
        <h1>Moqentra</h1>
        <label>
          Tenant:
          <input
            type="text"
            value={scope.tenantId}
            onChange={(e) => setTenant(e.target.value)}
            data-testid="tenant-input"
          />
        </label>
      </header>
      <main>
        <p>Project: {scope.projectId ?? "none"}</p>
      </main>
    </div>
  );
}
