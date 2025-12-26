import { useState, useEffect, type ReactNode } from "react";
import { AuthContext, createAuthValue } from "@/hooks/use-auth";
import { api } from "@/lib/api";

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const token = api.getToken();
    if (token) {
      api.validateToken().then((valid) => {
        setIsAuthenticated(valid);
        if (!valid) {
          api.setToken(null);
        }
        setLoading(false);
      });
    } else {
      setLoading(false);
    }
  }, []);

  const authValue = createAuthValue(isAuthenticated, setIsAuthenticated);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary" />
      </div>
    );
  }

  return (
    <AuthContext.Provider value={authValue}>{children}</AuthContext.Provider>
  );
}
