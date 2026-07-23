import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useAuth } from "./AuthContext";
import { useTenant } from "./TenantContext";

export function Shell() {
  const { t } = useTranslation();
  const { user, isAuthenticated, logout } = useAuth();
  const { scope, setTenant, setProject } = useTenant();

  return (
    <div>
      <header
        style={{
          display: "flex",
          alignItems: "center",
          gap: "var(--space-md)",
          padding: "var(--space-md)",
          borderBottom: "1px solid var(--color-border)",
        }}
      >
        <h1 style={{ margin: 0, fontSize: "1.25rem" }}>
          <Link to="/">{t("appName")}</Link>
        </h1>
        <nav aria-label="Main">
          <Link to="/projects">{t("projects")}</Link>
        </nav>
        <label style={{ marginLeft: "auto" }}>
          {t("tenant")}:
          <input
            type="text"
            value={scope.tenantId}
            onChange={(e) => {
              setTenant(e.target.value);
              setProject(undefined);
            }}
            data-testid="tenant-input"
            aria-label={t("tenant")}
          />
        </label>
        {isAuthenticated ? (
          <>
            <span>{user?.profile?.email ?? user?.profile?.sub}</span>
            <button type="button" onClick={logout}>
              {t("logout")}
            </button>
          </>
        ) : (
          <span>{t("login")}</span>
        )}
      </header>
      <main style={{ padding: "var(--space-md)" }}>
        <p>
          {t("project")}: {scope.projectId ?? t("selectProject")}
        </p>
      </main>
    </div>
  );
}
