import { useEffect } from "react";
import { useNavigate } from "react-router";

export function meta() {
  return [
    { title: "Notifications - Rivetr" },
    { name: "description", content: "Configure notification channels and preferences" },
  ];
}

export default function NotificationsRedirect() {
  const navigate = useNavigate();
  useEffect(() => {
    navigate("/settings/notifications", { replace: true });
  }, [navigate]);
  return null;
}
