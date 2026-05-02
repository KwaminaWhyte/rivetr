import { useParams } from "react-router";
import { EnvVarsTab } from "@/components/env-vars-tab";
import { LinkedDatabasesSection } from "@/components/linked-databases-section";

export function meta() {
  return [
    { title: "Environment Variables - Rivetr" },
    { name: "description", content: "Manage environment variables for your application" },
  ];
}

export default function AppEnvVarsPage() {
  const { id } = useParams();
  return (
    <div className="space-y-4">
      <EnvVarsTab appId={id!} />
      <LinkedDatabasesSection appId={id!} />
    </div>
  );
}
