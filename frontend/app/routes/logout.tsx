import { useEffect } from "react";
import { useNavigate } from "react-router";
import { getAuthToken, clearAuthToken } from "@/lib/auth";

export function meta() {
  return [{ title: "Logging out - Rivetr" }];
}

export default function LogoutPage() {
  const navigate = useNavigate();

  useEffect(() => {
    async function performLogout() {
      const token = getAuthToken();
      try {
        // Call logout endpoint to clear server-side session
        await fetch("/api/auth/logout", {
          method: "POST",
          headers: token ? { Authorization: `Bearer ${token}` } : {},
        });
      } catch {
        // Ignore errors - we still want to redirect to login
      }

      // Clear local token and redirect
      clearAuthToken();
      navigate("/login", { replace: true });
    }

    performLogout();
  }, [navigate]);

  return (
    <div className="flex h-screen items-center justify-center">
      <div className="animate-pulse text-muted-foreground">Logging out...</div>
    </div>
  );
}
