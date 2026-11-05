import { useOutletContext } from "react-router";
import { DatabaseSettingsSections } from "@/components/databases/database-settings-sections";
import type { ManagedDatabase } from "@/types/api";

export function meta() {
  return [
    { title: "Database resources - Rivetr" },
    { name: "description", content: "CPU and memory limits" },
  ];
}

export default function DatabaseResourcesSection() {
  const { database } = useOutletContext<{ database: ManagedDatabase }>();
  return <DatabaseSettingsSections database={database} sections={["resources"]} />;
}
