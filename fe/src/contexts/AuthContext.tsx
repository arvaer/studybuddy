import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from "react";
import { apiGet, apiPost } from "@/lib/api";

export interface User {
  id: string;
  email: string;
  displayName: string;
  avatar?: string;
  createdAt: Date;
}

interface AuthContextType {
  user: User | null;
  isLoading: boolean;
  login: (email: string, password: string) => Promise<void>;
  signup: (email: string, password: string, displayName: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | null>(null);

interface UserDTO {
  id: string;
  email: string;
  displayName: string;
  avatar?: string;
  createdAt: string;
}

interface AuthDTO {
  user: UserDTO;
  accessToken: string;
}

function toUser(dto: UserDTO): User {
  return { ...dto, createdAt: new Date(dto.createdAt) };
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Restore session from cookie on mount
  useEffect(() => {
    apiGet<UserDTO>("/api/auth/me")
      .then((dto) => setUser(toUser(dto)))
      .catch(() => setUser(null))
      .finally(() => setIsLoading(false));
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    setIsLoading(true);
    try {
      const data = await apiPost<AuthDTO>("/api/auth/login", { email, password });
      setUser(toUser(data.user));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const signup = useCallback(async (email: string, password: string, displayName: string) => {
    setIsLoading(true);
    try {
      const data = await apiPost<AuthDTO>("/api/auth/signup", { email, password, displayName });
      setUser(toUser(data.user));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const logout = useCallback(async () => {
    await apiPost("/api/auth/logout");
    setUser(null);
  }, []);

  return (
    <AuthContext.Provider value={{ user, isLoading, login, signup, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within AuthProvider");
  return ctx;
}
