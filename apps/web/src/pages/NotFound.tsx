import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";

export function NotFound() {
  const { t } = useTranslation();
  return (
    <main>
      <h1>{t("notFound")}</h1>
      <Link to="/">{t("goHome")}</Link>
    </main>
  );
}
