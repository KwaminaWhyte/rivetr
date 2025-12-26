import { useState, useEffect, type ReactNode } from "react";
import { AuthContext, createAuthValue } from "@/hooks/use-auth";
import { api } from "@/lib/api";

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [needsSetup, setNeedsSetup] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function checkAuth() {
      try {
        // First check if setup is needed
        const setupStatus = await api.checkSetupStatus();
        setNeedsSetup(setupStatus.needs_setup);

        // If setup is needed, no need to check auth
        if (setupStatus.needs_setup) {
          setLoading(false);
          return;
        }

        // Check if user has a valid token
        const token = api.getToken();
        if (token) {
          const valid = await api.validateToken();
          setIsAuthenticated(valid);
          if (!valid) {
            api.setToken(null);
          }
        }
      } catch (error) {
        console.error("Auth check failed:", error);
      } finally {
        setLoading(false);
      }
    }

    checkAuth();
  }, []);

  const authValue = createAuthValue(isAuthenticated, setIsAuthenticated, needsSetup);

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
