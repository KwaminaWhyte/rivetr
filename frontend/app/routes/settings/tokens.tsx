import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export default function SettingsTokensPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">API Tokens</h1>
          <p className="text-muted-foreground">
            Manage API tokens for programmatic access
          </p>
        </div>
        <Button disabled>Create Token</Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>API Tokens</CardTitle>
          <CardDescription>
            API tokens allow you to authenticate with the Rivetr API programmatically.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground py-4 text-center">
            API token management coming in a future update.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
