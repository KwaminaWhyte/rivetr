import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

export function SettingsTokensPage() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">API Tokens</h1>
        <Button disabled>Generate Token</Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Active Tokens</CardTitle>
          <CardDescription>
            Manage API tokens for programmatic access to Rivetr.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Created</TableHead>
                <TableHead>Last Used</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow>
                <TableCell className="font-medium">Admin Token</TableCell>
                <TableCell>System default</TableCell>
                <TableCell>-</TableCell>
                <TableCell>
                  <Badge>Active</Badge>
                </TableCell>
                <TableCell className="text-right">
                  <Button variant="ghost" size="sm" disabled>
                    Revoke
                  </Button>
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
          <p className="mt-4 text-xs text-muted-foreground">
            Token management is configured via the rivetr.toml configuration file.
            UI-based token management coming in a future update.
          </p>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>API Documentation</CardTitle>
          <CardDescription>
            Learn how to use the Rivetr API.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <span className="font-medium">Authentication</span>
            <p className="text-sm text-muted-foreground">
              Include your token in the Authorization header:
            </p>
            <code className="block rounded bg-muted px-3 py-2 text-sm">
              Authorization: Bearer your-token-here
            </code>
          </div>
          <div className="space-y-2">
            <span className="font-medium">Base URL</span>
            <code className="block rounded bg-muted px-3 py-2 text-sm">
              /api
            </code>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
