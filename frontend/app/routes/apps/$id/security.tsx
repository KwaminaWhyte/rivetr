import { useState } from "react";
import { useParams } from "react-router";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { aiApi } from "@/lib/api/ai";
import { Shield, ShieldCheck, Sparkles, RefreshCw, AlertTriangle, Info } from "lucide-react";

export function meta() {
  return [
    { title: "Security Scan - Rivetr" },
    { name: "description", content: "AI-powered security and compliance scan for this application" },
  ];
}

type Severity = "critical" | "high" | "medium" | "low";

interface Finding {
  severity: Severity;
  category: string;
  title: string;
  description: string;
  recommendation: string;
}

interface ScanResult {
  app_id: string;
  app_name: string;
  findings: Finding[];
  critical: number;
  high: number;
  medium: number;
  low: number;
  ai_summary: string | null;
}

function severityBadgeClass(severity: Severity): string {
  switch (severity) {
    case "critical":
      return "bg-red-600 text-white";
    case "high":
      return "bg-orange-500 text-white";
    case "medium":
      return "bg-yellow-500 text-white";
    case "low":
      return "bg-gray-400 text-white";
  }
}

function severityLabel(severity: Severity): string {
  return severity.charAt(0).toUpperCase() + severity.slice(1);
}

export default function AppSecurityPage() {
  const { id: appId } = useParams<{ id: string }>();
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [hasScanned, setHasScanned] = useState(false);
  const [unavailable, setUnavailable] = useState(false);

  const handleScan = async () => {
    if (!appId) return;
    setIsLoading(true);
    setUnavailable(false);
    setScanResult(null);
    try {
      const result = await aiApi.scanAppSecurity(appId);
      setScanResult(result);
      setHasScanned(true);
    } catch (error) {
      const msg = error instanceof Error ? error.message : "";
      if (
        msg.includes("503") ||
        msg.toLowerCase().includes("not configured") ||
        msg.toLowerCase().includes("unavailable")
      ) {
        setUnavailable(true);
        setHasScanned(true);
      } else {
        toast.error(msg || "Failed to run security scan");
      }
    } finally {
      setIsLoading(false);
    }
  };

  const totalFindings = scanResult
    ? scanResult.critical + scanResult.high + scanResult.medium + scanResult.low
    : 0;

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold flex items-center gap-2">
            <Shield className="h-5 w-5 text-blue-500" />
            Security &amp; Compliance
          </h2>
          <p className="text-sm text-muted-foreground">
            AI-powered security scan to identify vulnerabilities and compliance issues.
          </p>
        </div>
        <Button onClick={handleScan} disabled={isLoading} className="gap-2">
          <Sparkles className="h-4 w-4" />
          {isLoading ? "Scanning..." : "Scan Now"}
        </Button>
      </div>

      {/* Loading skeletons */}
      {isLoading && (
        <div className="space-y-4">
          <Card>
            <CardContent className="pt-6 space-y-3">
              <Skeleton className="h-4 w-3/4" />
              <Skeleton className="h-4 w-1/2" />
            </CardContent>
          </Card>
          {[1, 2, 3].map((i) => (
            <Card key={i}>
              <CardHeader>
                <Skeleton className="h-4 w-1/3" />
              </CardHeader>
              <CardContent className="space-y-2">
                <Skeleton className="h-3 w-full" />
                <Skeleton className="h-3 w-4/5" />
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* AI not configured */}
      {hasScanned && unavailable && (
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground flex items-center gap-2">
              <Shield className="h-4 w-4" />
              AI not configured. Enable an AI provider in instance settings to run security scans.
            </p>
          </CardContent>
        </Card>
      )}

      {/* Scan Results */}
      {hasScanned && scanResult && !isLoading && (
        <>
          {/* Summary bar */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Scan Summary</CardTitle>
              {scanResult.app_name && (
                <CardDescription>
                  Results for <span className="font-medium">{scanResult.app_name}</span>
                </CardDescription>
              )}
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex flex-wrap gap-3">
                <div className="flex items-center gap-2 rounded-lg border px-4 py-2">
                  <span className="text-sm text-muted-foreground">Critical</span>
                  <Badge className="bg-red-600 text-white">{scanResult.critical}</Badge>
                </div>
                <div className="flex items-center gap-2 rounded-lg border px-4 py-2">
                  <span className="text-sm text-muted-foreground">High</span>
                  <Badge className="bg-orange-500 text-white">{scanResult.high}</Badge>
                </div>
                <div className="flex items-center gap-2 rounded-lg border px-4 py-2">
                  <span className="text-sm text-muted-foreground">Medium</span>
                  <Badge className="bg-yellow-500 text-white">{scanResult.medium}</Badge>
                </div>
                <div className="flex items-center gap-2 rounded-lg border px-4 py-2">
                  <span className="text-sm text-muted-foreground">Low</span>
                  <Badge className="bg-gray-400 text-white">{scanResult.low}</Badge>
                </div>
              </div>

              {/* AI Summary */}
              {scanResult.ai_summary && (
                <div className="rounded-lg border border-blue-200 bg-blue-50/50 dark:bg-blue-950/10 p-4 space-y-1">
                  <p className="text-xs font-medium text-blue-700 dark:text-blue-400 flex items-center gap-1">
                    <Sparkles className="h-3.5 w-3.5" />
                    AI Summary
                  </p>
                  <p className="text-sm">{scanResult.ai_summary}</p>
                </div>
              )}

              <Button
                variant="outline"
                size="sm"
                className="gap-2"
                onClick={handleScan}
                disabled={isLoading}
              >
                <RefreshCw className="h-4 w-4" />
                Re-scan
              </Button>
            </CardContent>
          </Card>

          {/* No findings state */}
          {totalFindings === 0 && (
            <Card>
              <CardContent className="pt-8 pb-8 flex flex-col items-center gap-3 text-center">
                <ShieldCheck className="h-12 w-12 text-green-500" />
                <div>
                  <p className="font-medium text-green-700 dark:text-green-400">No issues found</p>
                  <p className="text-sm text-muted-foreground">
                    Your app passed all security checks. Keep it up!
                  </p>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Findings list */}
          {scanResult.findings.length > 0 && (
            <div className="space-y-3">
              <p className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                Findings ({scanResult.findings.length})
              </p>
              {scanResult.findings.map((finding, i) => (
                <Card key={i}>
                  <CardHeader className="pb-2">
                    <div className="flex items-start justify-between gap-3">
                      <div className="space-y-1">
                        <CardTitle className="text-base">{finding.title}</CardTitle>
                        <p className="text-xs text-muted-foreground">{finding.category}</p>
                      </div>
                      <Badge className={severityBadgeClass(finding.severity)}>
                        {severityLabel(finding.severity)}
                      </Badge>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <p className="text-sm text-muted-foreground">{finding.description}</p>
                    <div className="rounded-lg border border-blue-200 bg-blue-50/50 dark:bg-blue-950/10 p-3 flex items-start gap-2">
                      <Info className="h-4 w-4 text-blue-500 shrink-0 mt-0.5" />
                      <p className="text-sm">{finding.recommendation}</p>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </>
      )}

      {/* Pre-scan empty state */}
      {!hasScanned && !isLoading && (
        <Card>
          <CardContent className="pt-8 pb-8 flex flex-col items-center gap-3 text-center">
            <Shield className="h-12 w-12 text-muted-foreground/40" />
            <div>
              <p className="font-medium">Run a security scan</p>
              <p className="text-sm text-muted-foreground">
                Click "Scan Now" to check your app for security vulnerabilities and compliance issues.
              </p>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
