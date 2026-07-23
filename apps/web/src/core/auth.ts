import {
  InMemoryWebStorage,
  User,
  UserManager,
  UserManagerSettings,
  WebStorageStateStore,
} from "oidc-client-ts";

function buildSettings(): UserManagerSettings {
  const origin = typeof window !== "undefined" ? window.location.origin : "";
  const authority = import.meta.env?.VITE_OIDC_ISSUER || "";
  const client_id = import.meta.env?.VITE_OIDC_CLIENT_ID || "moqentra-web";
  const redirect_uri =
    import.meta.env?.VITE_OIDC_REDIRECT_URI || `${origin}/oidc/callback`;
  const post_logout_redirect_uri =
    import.meta.env?.VITE_OIDC_POST_LOGOUT_URI || `${origin}/login`;
  const scope = import.meta.env?.VITE_OIDC_SCOPE || "openid profile email";
  const store = new WebStorageStateStore({ store: new InMemoryWebStorage() });

  return {
    authority,
    client_id,
    redirect_uri,
    post_logout_redirect_uri,
    response_type: "code",
    scope,
    userStore: store,
    stateStore: store,
  };
}

let userManagerInstance: UserManager | null = null;

export function getUserManager(): UserManager {
  if (!userManagerInstance) {
    userManagerInstance = new UserManager(buildSettings());
  }
  return userManagerInstance;
}

let inMemoryAccessToken: string | null = null;

export const tokenManager = {
  setAccessToken(token: string | null) {
    inMemoryAccessToken = token;
  },
  getAccessToken(): string | undefined {
    return inMemoryAccessToken ?? undefined;
  },
  clear() {
    inMemoryAccessToken = null;
  },
};

export async function login(): Promise<void> {
  return getUserManager().signinRedirect();
}

export async function handleLoginCallback(url?: string): Promise<User> {
  const user = await getUserManager().signinRedirectCallback(url);
  tokenManager.setAccessToken(user.access_token);
  return user;
}

export async function logout(): Promise<void> {
  tokenManager.clear();
  return getUserManager().signoutRedirect();
}

export async function loadUser(): Promise<User | null> {
  const user = await getUserManager().getUser();
  tokenManager.setAccessToken(user?.access_token ?? null);
  return user;
}
