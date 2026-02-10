import { createContext, useContext, useState, useCallback, ReactNode } from "react";

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
  logout: () => void;
}

const AuthContext = createContext<AuthContextType | null>(null);

const MOCK_USERS: Record<string, { password: string; user: User }> = {
  "demo@studybuddy.app": {
    password: "demo1234",
    user: {
      id: "usr_1",
      email: "demo@studybuddy.app",
      displayName: "Alex Chen",
      createdAt: new Date("2025-09-01"),
    },
  },
};

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(() => {
    const stored = localStorage.getItem("sb_user");
    return stored ? JSON.parse(stored) : null;
  });
  const [isLoading, setIsLoading] = useState(false);

  const login = useCallback(async (email: string, password: string) => {
    setIsLoading(true);
    await new Promise((r) => setTimeout(r, 800));
    const entry = MOCK_USERS[email];
    if (!entry || entry.password !== password) {
      setIsLoading(false);
      throw new Error("Invalid email or password");
    }
    localStorage.setItem("sb_user", JSON.stringify(entry.user));
    setUser(entry.user);
    setIsLoading(false);
  }, []);

  const signup = useCallback(async (email: string, password: string, displayName: string) => {
    setIsLoading(true);
    await new Promise((r) => setTimeout(r, 800));
    if (MOCK_USERS[email]) {
      setIsLoading(false);
      throw new Error("Email already in use");
    }
    const newUser: User = {
      id: `usr_${Date.now()}`,
      email,
      displayName,
      createdAt: new Date(),
    };
    MOCK_USERS[email] = { password, user: newUser };
    localStorage.setItem("sb_user", JSON.stringify(newUser));
    setUser(newUser);
    setIsLoading(false);
  }, []);

  const logout = useCallback(() => {
    localStorage.removeItem("sb_user");
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
