import { useOutletContext } from "react-router";
import { ContainerFileBrowser } from "@/components/container-file-browser";
import type { ManagedDatabase } from "@/types/api";

export function meta() {
  return [
    { title: "Files - Rivetr" },
    { name: "description", content: "Browse files inside the database container" },
  ];
}

export default function DatabaseFilesTab() {
  const { database } = useOutletContext<{ database: ManagedDatabase }>();
  return (
    <ContainerFileBrowser
      target={{ kind: "database", id: database.id }}
      unavailableMessage={
        database.status === "running" ? null : "Start the database to browse its files."
      }
    />
  );
}
