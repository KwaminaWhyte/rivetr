// Client-side authentication utilities for SPA mode
// Validates auth state by checking the token against the API

import { useEffect, useState } from "react";
import { useNavigate } from "react-router";

const TOKEN_KEY = "rivetr_auth_token";

interface AuthState {
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
}

/**
 * Store auth token after successful login
 */
export function setAuthToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
}

/**
 * Get stored auth token
 */
export function getAuthToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

/**
 * Clear auth token on logout
 */
export function clearAuthToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

/**
 * Validates the current session by calling the backend API
 * Returns true if authenticated, false otherwise
 */
export async function validateAuth(): Promise<boolean> {
  try {
    const token = getAuthToken();
    if (!token) return false;

    const response = await fetch("/api/auth/validate", {
      headers: {
        "Authorization": `Bearer ${token}`,
      },
    });
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Checks if initial setup is required
 */
export async function checkSetupStatus(): Promise<boolean> {
  try {
    const response = await fetch("/api/auth/setup-status", {
      credentials: "include",
    });
    const data = await response.json();
    return data.needs_setup;
  } catch {
    return false;
  }
}

/**
 * Hook for protected routes - redirects to login if not authenticated
 * Use this in layout components to protect child routes
 */
export function useRequireAuth(): AuthState {
  const [state, setState] = useState<AuthState>({
    isAuthenticated: false,
    isLoading: true,
    error: null,
  });
  const navigate = useNavigate();

  useEffect(() => {
    let mounted = true;

    async function checkAuth() {
      try {
        // First check if setup is needed
        const needsSetup = await checkSetupStatus();
        if (needsSetup) {
          navigate("/setup", { replace: true });
          return;
        }

        // Then validate auth
        const isAuthenticated = await validateAuth();
        if (!mounted) return;

        if (isAuthenticated) {
          setState({ isAuthenticated: true, isLoading: false, error: null });
        } else {
          navigate("/login", { replace: true });
        }
      } catch (error) {
        if (!mounted) return;
        setState({
          isAuthenticated: false,
          isLoading: false,
          error: error instanceof Error ? error.message : "Authentication failed",
        });
        navigate("/login", { replace: true });
      }
    }

    checkAuth();

    return () => {
      mounted = false;
    };
  }, [navigate]);

  return state;
}

/**
 * Hook for public routes - redirects to dashboard if already authenticated
 * Use this in login/setup pages
 */
export function usePublicRoute(): AuthState {
  const [state, setState] = useState<AuthState>({
    isAuthenticated: false,
    isLoading: true,
    error: null,
  });
  const navigate = useNavigate();

  useEffect(() => {
    let mounted = true;

    async function checkAuth() {
      try {
        const isAuthenticated = await validateAuth();
        if (!mounted) return;

        if (isAuthenticated) {
          // Already authenticated, redirect to dashboard
          navigate("/", { replace: true });
        } else {
          setState({ isAuthenticated: false, isLoading: false, error: null });
        }
      } catch {
        if (!mounted) return;
        setState({ isAuthenticated: false, isLoading: false, error: null });
      }
    }

    checkAuth();

    return () => {
      mounted = false;
    };
  }, [navigate]);

  return state;
}
