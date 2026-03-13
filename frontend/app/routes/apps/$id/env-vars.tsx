import { useOutletContext } from "react-router";
import { useParams } from "react-router";
import { EnvVarsTab } from "@/components/env-vars-tab";
import type { App } from "@/types/api";

export function meta() {
  return [
    { title: "Environment Variables - Rivetr" },
    { name: "description", content: "Manage environment variables for your application" },
  ];
}

interface OutletContext {
  app: App;
}

export default function AppEnvVarsPage() {
  const { id } = useParams();
  return <EnvVarsTab appId={id!} />;
}
