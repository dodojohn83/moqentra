import { useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useAuth } from "../core/AuthContext";

export function Login() {
  const { t } = useTranslation();
  const { login, isAuthenticated } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const from = (location.state as { from?: { pathname?: string } } | undefined)?.from?.pathname || "/";

  useEffect(() => {
    if (isAuthenticated) {
      navigate(from, { replace: true });
    }
  }, [isAuthenticated, navigate, from]);

  return (
    <main style={{ maxWidth: "24rem", margin: "var(--space-xl) auto", padding: "var(--space-lg)" }}>
      <h1>{t("appName")}</h1>
      <button type="button" onClick={login}>
        {t("login")}
      </button>
    </main>
  );
}
