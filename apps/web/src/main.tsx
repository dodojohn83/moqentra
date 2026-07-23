import { StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import "./index.css";
import "./core/i18n";
import { AuthProvider } from "./core/AuthContext";
import { AuthErrorBoundary } from "./core/AuthErrorBoundary";
import { AppRoutes } from "./core/AppRoutes";
import { Shell } from "./core/Shell";
import { TenantProvider } from "./core/TenantContext";

function App() {
  return (
    <BrowserRouter>
      <AuthErrorBoundary>
        <AuthProvider>
          <TenantProvider initialTenantId="default">
            <Suspense fallback={<div aria-live="polite">Loading…</div>}>
              <Shell />
              <AppRoutes />
            </Suspense>
          </TenantProvider>
        </AuthProvider>
      </AuthErrorBoundary>
    </BrowserRouter>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
