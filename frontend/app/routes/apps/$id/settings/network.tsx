import { useState } from "react";
import { useOutletContext } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { DomainManagementCard } from "@/components/domain-management-card";
import { NetworkConfigCard } from "@/components/network-config-card";
import { ContainerLabelsCard } from "@/components/container-labels-card";
import { api } from "@/lib/api";
import type { App, UpdateAppRequest } from "@/types/api";

export default function AppSettingsNetwork() {
  const { app } = useOutletContext<{ app: App }>();
  const queryClient = useQueryClient();
  const [isSavingNetwork, setIsSavingNetwork] = useState(false);
  const [isSavingDomains, setIsSavingDomains] = useState(false);
  const [isSavingLabels, setIsSavingLabels] = useState(false);

  const handleSaveNetworkConfig = async (updates: UpdateAppRequest) => {
    setIsSavingNetwork(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingNetwork(false);
    }
  };

  const handleSaveDomainConfig = async (updates: UpdateAppRequest) => {
    setIsSavingDomains(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingDomains(false);
    }
  };

  const handleSaveContainerLabels = async (updates: UpdateAppRequest) => {
    setIsSavingLabels(true);
    try {
      await api.updateApp(app.id, updates);
      queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    } finally {
      setIsSavingLabels(false);
    }
  };

  return (
    <div className="space-y-6">
      <DomainManagementCard app={app} onSave={handleSaveDomainConfig} isSaving={isSavingDomains} />
      <NetworkConfigCard app={app} onSave={handleSaveNetworkConfig} isSaving={isSavingNetwork} />
      <ContainerLabelsCard app={app} onSave={handleSaveContainerLabels} isSaving={isSavingLabels} />
    </div>
  );
}
