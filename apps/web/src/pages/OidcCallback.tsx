import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useAuth } from "../core/AuthContext";

export function OidcCallback() {
  const { t } = useTranslation();
  const { handleCallback } = useAuth();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    handleCallback(window.location.href)
      .then(() => navigate("/", { replace: true }))
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [handleCallback, navigate]);

  if (error) {
    return (
      <main role="alert" aria-live="assertive">
        <h1>{t("error")}</h1>
        <pre>{error}</pre>
      </main>
    );
  }

  return <main aria-live="polite">{t("loading")}</main>;
}
