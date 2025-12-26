import { BrowserRouter, Routes, Route, Navigate } from "react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AuthProvider } from "@/components/providers/AuthProvider";
import { ProtectedRoute } from "@/components/providers/ProtectedRoute";
import { DashboardLayout } from "@/components/layout/DashboardLayout";
import { Toaster } from "@/components/ui/sonner";
import { LoginPage } from "@/pages/Login";
import { SetupPage } from "@/pages/Setup";
import { DashboardPage } from "@/pages/Dashboard";
import { AppsPage } from "@/pages/Apps";
import { AppDetailPage } from "@/pages/AppDetail";
import { NewAppPage } from "@/pages/NewApp";
import { DeploymentsPage } from "@/pages/Deployments";
import { SettingsPage } from "@/pages/Settings";
import { SettingsWebhooksPage } from "@/pages/SettingsWebhooks";
import { SettingsTokensPage } from "@/pages/SettingsTokens";
import { SettingsSshKeysPage } from "@/pages/SettingsSshKeys";
import "./index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60,
      retry: 1,
    },
  },
});

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AuthProvider>
          <Routes>
            <Route path="/login" element={<LoginPage />} />
            <Route path="/setup" element={<SetupPage />} />
            <Route element={<ProtectedRoute />}>
              <Route element={<DashboardLayout />}>
                <Route path="/" element={<DashboardPage />} />
                <Route path="/apps" element={<AppsPage />} />
                <Route path="/apps/new" element={<NewAppPage />} />
                <Route path="/apps/:id" element={<AppDetailPage />} />
                <Route path="/deployments" element={<DeploymentsPage />} />
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="/settings/webhooks" element={<SettingsWebhooksPage />} />
                <Route path="/settings/tokens" element={<SettingsTokensPage />} />
                <Route path="/settings/ssh-keys" element={<SettingsSshKeysPage />} />
              </Route>
            </Route>
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
          <Toaster position="top-right" richColors />
        </AuthProvider>
      </BrowserRouter>
    </QueryClientProvider>
  );
}

export default App;
