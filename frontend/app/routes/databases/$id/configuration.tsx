import { useOutletContext } from "react-router";
import { DatabaseSettingsSections } from "@/components/databases/database-settings-sections";
import type { ManagedDatabase } from "@/types/api";

export function meta() {
  return [
    { title: "Database configuration - Rivetr" },
    { name: "description", content: "Image, init commands and SSL" },
  ];
}

export default function DatabaseConfigurationSection() {
  const { database } = useOutletContext<{ database: ManagedDatabase }>();
  return <DatabaseSettingsSections database={database} sections={["configuration"]} />;
}
