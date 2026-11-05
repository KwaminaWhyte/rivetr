import { TeamPage } from "@/components/teams/team-detail";

export function meta() {
  return [
    { title: "Team members - Rivetr" },
    { name: "description", content: "Members and invitations of the current team" },
  ];
}

export default function TeamSectionPage() {
  return <TeamPage section="members" />;
}
