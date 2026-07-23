import { createContext, useContext, useMemo, useState } from "react";
import { clearTenant } from "./queryCache";

export interface TenantScope {
  tenantId: string;
  projectId?: string;
}

interface TenantContextValue {
  scope: TenantScope;
  setTenant: (tenantId: string) => void;
  setProject: (projectId: string | undefined) => void;
}

const TenantContext = createContext<TenantContextValue | null>(null);

export function TenantProvider({
  initialTenantId,
  children,
}: {
  initialTenantId: string;
  children: React.ReactNode;
}) {
  const [scope, setScope] = useState<TenantScope>({ tenantId: initialTenantId });

  const value = useMemo(
    () => ({
      scope,
      setTenant: (tenantId: string) => {
        // Cancel cached work and isolate the next tenant (R1-WEB-003).
        clearTenant(scope.tenantId);
        setScope({ tenantId });
      },
      setProject: (projectId: string | undefined) =>
        setScope((prev) => ({ ...prev, projectId })),
    }),
    [scope],
  );

  return <TenantContext.Provider value={value}>{children}</TenantContext.Provider>;
}

export function useTenant(): TenantContextValue {
  const ctx = useContext(TenantContext);
  if (!ctx) throw new Error("useTenant must be used within TenantProvider");
  return ctx;
}
