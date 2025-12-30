import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export default function SettingsGitProvidersPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Git Providers</h1>
          <p className="text-muted-foreground">
            Connect Git providers for OAuth authentication
          </p>
        </div>
        <Button disabled>Add Provider</Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Connected Providers</CardTitle>
          <CardDescription>
            OAuth connections allow you to access private repositories without SSH keys.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground py-4 text-center">
            Git provider OAuth coming in a future update.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
