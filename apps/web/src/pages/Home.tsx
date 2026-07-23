import { useTranslation } from "react-i18next";

export function Home() {
  const { t } = useTranslation();
  return (
    <main>
      <h1>{t("home")}</h1>
      <p>Welcome to Moqentra.</p>
    </main>
  );
}
