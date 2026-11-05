import { useOutletContext } from "react-router";
import { DatabaseSettingsSections } from "@/components/databases/database-settings-sections";
import type { ManagedDatabase } from "@/types/api";

export function meta() {
  return [
    { title: "Database operations - Rivetr" },
    { name: "description", content: "Move to another project or clone this database" },
  ];
}

export default function DatabaseOperationsSection() {
  const { database } = useOutletContext<{ database: ManagedDatabase }>();
  return <DatabaseSettingsSections database={database} sections={["operations"]} />;
}
