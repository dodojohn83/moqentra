import { useMemo } from "react";
import { Configuration, DefaultApi } from "../generated/api";
import { tokenManager } from "./auth";
import { defaultApiFetch } from "./apiClient";

const basePath = import.meta.env.VITE_API_BASE_URL || "";

export function useApi(): DefaultApi {
  return useMemo(
    () =>
      new DefaultApi(
        new Configuration({
          basePath,
          fetchApi: defaultApiFetch,
          accessToken: () => Promise.resolve(tokenManager.getAccessToken() ?? ""),
        }),
      ),
    [],
  );
}
