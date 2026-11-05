import { useOutletContext } from "react-router";
import { DatabaseSettingsSections } from "@/components/databases/database-settings-sections";
import type { ManagedDatabase } from "@/types/api";

export function meta() {
  return [
    { title: "Danger zone - Rivetr" },
    { name: "description", content: "Delete this database" },
  ];
}

export default function DatabaseDangerSection() {
  const { database } = useOutletContext<{ database: ManagedDatabase }>();
  return <DatabaseSettingsSections database={database} sections={["danger"]} />;
}
