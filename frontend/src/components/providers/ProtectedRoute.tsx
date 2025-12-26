import { Navigate, Outlet } from "react-router";
import { useAuth } from "@/hooks/use-auth";

export function ProtectedRoute() {
  const { isAuthenticated } = useAuth();

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return <Outlet />;
}
