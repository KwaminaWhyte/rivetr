import { useParams } from "react-router";
import { EnvVarsTab } from "@/components/env-vars-tab";
import { LinkedDatabasesSection } from "@/components/linked-databases-section";
import { SecretManagerSection } from "@/components/secret-managers/secret-manager-section";

export function meta() {
  return [
    { title: "Environment variables - Rivetr" },
    { name: "description", content: "Manage environment variables for your application" },
  ];
}

export default function AppEnvVarsPage() {
  const { id } = useParams();
  return (
    <div className="space-y-4">
      <EnvVarsTab appId={id!} />
      <SecretManagerSection ownerType="app" ownerId={id!} />
      <LinkedDatabasesSection appId={id!} />
    </div>
  );
}
