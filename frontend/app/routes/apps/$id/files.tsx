import { useOutletContext } from "react-router";
import { ContainerFileBrowser } from "@/components/container-file-browser";
import type { App } from "@/types/api";

export function meta() {
  return [
    { title: "Files - Rivetr" },
    { name: "description", content: "Browse files inside the app container" },
  ];
}

export default function AppFilesTab() {
  const { app } = useOutletContext<{ app: App }>();
  return <ContainerFileBrowser target={{ kind: "app", id: app.id }} />;
}
