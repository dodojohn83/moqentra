import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Shell } from "./core/Shell";
import { TenantProvider } from "./core/TenantContext";

function App() {
  return (
    <TenantProvider initialTenantId="default">
      <Shell />
    </TenantProvider>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
