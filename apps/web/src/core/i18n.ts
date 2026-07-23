import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const resources = {
  en: {
    translation: {
      appName: "Moqentra",
      login: "Sign in",
      logout: "Sign out",
      home: "Home",
      projects: "Projects",
      tenant: "Tenant",
      project: "Project",
      selectProject: "Select project",
      loading: "Loading…",
      error: "Something went wrong",
      retry: "Retry",
      notFound: "Page not found",
      goHome: "Go home",
    },
  },
  zh: {
    translation: {
      appName: "Moqentra",
      login: "登录",
      logout: "退出",
      home: "首页",
      projects: "项目",
      tenant: "租户",
      project: "项目",
      selectProject: "选择项目",
      loading: "加载中…",
      error: "出错了",
      retry: "重试",
      notFound: "页面不存在",
      goHome: "返回首页",
    },
  },
};

i18n.use(initReactI18next).init({
  resources,
  lng: "en",
  fallbackLng: "en",
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;
