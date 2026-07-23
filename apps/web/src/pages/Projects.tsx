import { useTranslation } from "react-i18next";

export function Projects() {
  const { t } = useTranslation();
  return (
    <main>
      <h1>{t("projects")}</h1>
      <p>Project list will be loaded here.</p>
    </main>
  );
}
