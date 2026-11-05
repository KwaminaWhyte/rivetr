import { InstanceSettings } from "@/components/settings/instance-settings";

export function meta() {
  return [
    { title: "AI - Settings - Rivetr" },
    { name: "description", content: "AI provider used for insights, diagnosis and security scans" },
  ];
}

export default function SettingsAiPage() {
  return <InstanceSettings aiOnly />;
}
