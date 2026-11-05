import { useOutletContext } from "react-router";
import { ServiceSettingsSections } from "@/components/services/service-settings-sections";
import type { Service } from "@/types/api";

export function meta() {
  return [
    { title: "Compose file - Rivetr" },
    { name: "description", content: "The Docker Compose file and raw compose mode" },
  ];
}

export default function ServiceComposeSection() {
  const { service } = useOutletContext<{ service: Service }>();
  return <ServiceSettingsSections service={service} sections={["compose"]} />;
}
