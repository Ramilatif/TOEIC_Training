import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";
import { api, getToken, setToken, type User } from "./api/client";

interface AuthState {
  user: User | null;
  loading: boolean;
  login: (pseudo: string) => Promise<void>;
  logout: () => Promise<void>;
  setUser: (u: User) => void;
}

const Ctx = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(!!getToken());

  useEffect(() => {
    if (!getToken()) return;
    api
      .me()
      .then(setUser)
      .catch(() => setToken(null))
      .finally(() => setLoading(false));
  }, []);

  const login = useCallback(async (pseudo: string) => {
    const r = await api.login(pseudo);
    setToken(r.token);
    setUser(r.user);
  }, []);

  const logout = useCallback(async () => {
    await api.logout().catch(() => undefined);
    setToken(null);
    setUser(null);
  }, []);

  return <Ctx.Provider value={{ user, loading, login, logout, setUser }}>{children}</Ctx.Provider>;
}

export function useAuth(): AuthState {
  const c = useContext(Ctx);
  if (!c) throw new Error("useAuth hors AuthProvider");
  return c;
}
