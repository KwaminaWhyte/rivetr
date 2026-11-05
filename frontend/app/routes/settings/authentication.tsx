import { OAuthProviderSettings } from "@/components/settings/oauth-providers";
import { SsoProviderSettings } from "@/components/settings/sso-providers";
import { useIsInstanceAdmin } from "@/lib/auth";

export function meta() {
  return [
    { title: "Authentication - Settings - Rivetr" },
    {
      name: "description",
      content: "OAuth login providers and SSO / OIDC identity providers for this instance",
    },
  ];
}

/** OAuth providers and, for instance admins, SSO / OIDC providers. */
export default function SettingsAuthenticationPage() {
  const isAdmin = useIsInstanceAdmin();
  return (
    <div className="space-y-10">
      <OAuthProviderSettings />
      {isAdmin && <SsoProviderSettings />}
    </div>
  );
}
