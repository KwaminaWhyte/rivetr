import { api } from "@/lib/api";
import { createContext, useContext } from "react";

interface AuthContextType {
  isAuthenticated: boolean;
  login: (email: string, password: string) => Promise<boolean>;
  logout: () => void;
}

export const AuthContext = createContext<AuthContextType | null>(null);

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}

export function createAuthValue(
  isAuthenticated: boolean,
  setIsAuthenticated: (value: boolean) => void
): AuthContextType {
  return {
    isAuthenticated,
    login: async (email: string, password: string) => {
      try {
        await api.login(email, password);
        setIsAuthenticated(true);
        return true;
      } catch {
        return false;
      }
    },
    logout: () => {
      api.setToken(null);
      setIsAuthenticated(false);
    },
  };
}
