import { TeamPage } from "@/components/teams/team-detail";

export function meta() {
  return [
    { title: "Team variables - Rivetr" },
    { name: "description", content: "Shared variables for every app in the current team" },
  ];
}

export default function TeamSectionPage() {
  return <TeamPage section="variables" />;
}
