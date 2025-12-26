import { BrowserRouter, Routes, Route, Navigate } from "react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AuthProvider } from "@/components/providers/AuthProvider";
import { ThemeProvider } from "@/components/providers/ThemeProvider";
import { ProtectedRoute } from "@/components/providers/ProtectedRoute";
import { DashboardLayout } from "@/components/layout/DashboardLayout";
import { Toaster } from "@/components/ui/sonner";
import { LoginPage } from "@/pages/Login";
import { SetupPage } from "@/pages/Setup";
import { DashboardPage } from "@/pages/Dashboard";
import { AppDetailPage } from "@/pages/AppDetail";
import { NewAppPage } from "@/pages/NewApp";
import { DeploymentsPage } from "@/pages/Deployments";
import { SettingsPage } from "@/pages/Settings";
import { SettingsWebhooksPage } from "@/pages/SettingsWebhooks";
import { SettingsTokensPage } from "@/pages/SettingsTokens";
import { SettingsSshKeysPage } from "@/pages/SettingsSshKeys";
import { SettingsGitProvidersPage } from "@/pages/SettingsGitProviders";
import { ProjectsPage } from "@/pages/Projects";
import { ProjectDetailPage } from "@/pages/ProjectDetail";
import { MonitoringPage } from "@/pages/Monitoring";
import { NotificationsPage } from "@/pages/Notifications";
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
      <ThemeProvider>
        <BrowserRouter>
          <AuthProvider>
          <Routes>
            <Route path="/login" element={<LoginPage />} />
            <Route path="/setup" element={<SetupPage />} />
            <Route element={<ProtectedRoute />}>
              <Route element={<DashboardLayout />}>
                <Route path="/" element={<DashboardPage />} />
                {/* Projects - main entry point for apps */}
                <Route path="/projects" element={<ProjectsPage />} />
                <Route path="/projects/:id" element={<ProjectDetailPage />} />
                <Route path="/projects/:projectId/apps/new" element={<NewAppPage />} />
                {/* App detail - accessible via project or direct link */}
                <Route path="/apps/:id" element={<AppDetailPage />} />
                {/* Deployments - cross-project view */}
                <Route path="/deployments" element={<DeploymentsPage />} />
                {/* Monitoring */}
                <Route path="/monitoring" element={<MonitoringPage />} />
                <Route path="/monitoring/metrics" element={<MonitoringPage />} />
                <Route path="/monitoring/logs" element={<MonitoringPage />} />
                {/* Notifications */}
                <Route path="/notifications" element={<NotificationsPage />} />
                {/* Settings */}
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="/settings/webhooks" element={<SettingsWebhooksPage />} />
                <Route path="/settings/tokens" element={<SettingsTokensPage />} />
                <Route path="/settings/ssh-keys" element={<SettingsSshKeysPage />} />
                <Route path="/settings/git-providers" element={<SettingsGitProvidersPage />} />
                {/* Legacy redirects */}
                <Route path="/apps" element={<Navigate to="/projects" replace />} />
                <Route path="/apps/new" element={<Navigate to="/projects" replace />} />
              </Route>
            </Route>
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
          <Toaster position="top-right" richColors />
          </AuthProvider>
        </BrowserRouter>
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default App;
