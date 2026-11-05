import { useOutletContext } from "react-router";
import { ServiceSettingsSections } from "@/components/services/service-settings-sections";
import type { Service } from "@/types/api";

export function meta() {
  return [
    { title: "Service resources - Rivetr" },
    { name: "description", content: "CPU and memory limits for every container" },
  ];
}

export default function ServiceResourcesSection() {
  const { service } = useOutletContext<{ service: Service }>();
  return <ServiceSettingsSections service={service} sections={["resources"]} />;
}
