import { useOutletContext } from "react-router";
import { ServiceSettingsSections } from "@/components/services/service-settings-sections";
import type { Service } from "@/types/api";

export function meta() {
  return [
    { title: "Danger zone - Rivetr" },
    { name: "description", content: "Delete this service" },
  ];
}

export default function ServiceDangerSection() {
  const { service } = useOutletContext<{ service: Service }>();
  return <ServiceSettingsSections service={service} sections={["danger"]} />;
}
