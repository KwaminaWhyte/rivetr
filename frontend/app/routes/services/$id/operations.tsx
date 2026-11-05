import { useOutletContext } from "react-router";
import { ServiceSettingsSections } from "@/components/services/service-settings-sections";
import type { Service } from "@/types/api";

export function meta() {
  return [
    { title: "Service operations - Rivetr" },
    { name: "description", content: "Move or clone this service and import a database dump" },
  ];
}

export default function ServiceOperationsSection() {
  const { service } = useOutletContext<{ service: Service }>();
  return <ServiceSettingsSections service={service} sections={["operations", "dump"]} />;
}
