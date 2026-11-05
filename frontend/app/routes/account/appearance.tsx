import { AppearanceSettings } from "@/components/appearance/appearance-settings";
import { PageHeader } from "@/components/page";

export function meta() {
  return [
    { title: "Appearance - Account - Rivetr" },
    { name: "description", content: "Theme and colours of the dashboard for your account" },
  ];
}

export default function AccountAppearancePage() {
  return (
    <div className="space-y-6">
      <PageHeader title="Appearance" description="How the dashboard looks for you." />
      <AppearanceSettings />
    </div>
  );
}
