import { InstanceSettings } from "@/components/settings/instance-settings";

export function meta() {
  return [
    { title: "Settings - Rivetr" },
    { name: "description", content: "Configure your Rivetr instance settings" },
  ];
}

export default function SettingsGeneralPage() {
  return <InstanceSettings />;
}
