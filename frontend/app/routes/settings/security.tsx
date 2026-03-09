import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Alert,
  AlertDescription,
  AlertTitle,
} from "@/components/ui/alert";
import {
  ShieldCheck,
  ShieldOff,
  Copy,
  Download,
  AlertTriangle,
  CheckCircle2,
  Loader2,
} from "lucide-react";
import { twoFactorApi } from "@/lib/api/two-factor";
import type {
  TwoFactorSetupResponse,
  TwoFactorVerifyResponse,
} from "@/lib/api/two-factor";

export function meta() {
  return [
    { title: "Security Settings - Rivetr" },
    {
      name: "description",
      content: "Manage two-factor authentication and security settings",
    },
  ];
}

export default function SecuritySettingsPage() {
  const queryClient = useQueryClient();

  // Setup flow state
  const [setupStep, setSetupStep] = useState<
    "idle" | "scanning" | "verifying" | "recovery"
  >("idle");
  const [setupData, setSetupData] = useState<TwoFactorSetupResponse | null>(
    null
  );
  const [verifyCode, setVerifyCode] = useState("");
  const [recoveryCodes, setRecoveryCodes] = useState<string[]>([]);

  // Disable dialog state
  const [showDisableDialog, setShowDisableDialog] = useState(false);
  const [disableCode, setDisableCode] = useState("");

  // Query 2FA status
  const { data: status, isLoading } = useQuery({
    queryKey: ["2fa-status"],
    queryFn: () => twoFactorApi.getStatus(),
  });

  // Setup mutation
  const setupMutation = useMutation({
    mutationFn: () => twoFactorApi.setup(),
    onSuccess: (data) => {
      setSetupData(data);
      setSetupStep("scanning");
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to start 2FA setup"
      );
    },
  });

  // Verify mutation
  const verifyMutation = useMutation({
    mutationFn: (code: string) => twoFactorApi.verify(code),
    onSuccess: (data: TwoFactorVerifyResponse) => {
      setRecoveryCodes(data.recovery_codes);
      setSetupStep("recovery");
      queryClient.invalidateQueries({ queryKey: ["2fa-status"] });
      toast.success("Two-factor authentication enabled");
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Invalid verification code"
      );
      setVerifyCode("");
    },
  });

  // Disable mutation
  const disableMutation = useMutation({
    mutationFn: (code: string) => twoFactorApi.disable(code),
    onSuccess: () => {
      setShowDisableDialog(false);
      setDisableCode("");
      queryClient.invalidateQueries({ queryKey: ["2fa-status"] });
      toast.success("Two-factor authentication disabled");
    },
    onError: (error) => {
      toast.error(
        error instanceof Error ? error.message : "Failed to disable 2FA"
      );
    },
  });

  function handleCopyRecoveryCodes() {
    navigator.clipboard.writeText(recoveryCodes.join("\n"));
    toast.success("Recovery codes copied to clipboard");
  }

  function handleDownloadRecoveryCodes() {
    const content = [
      "Rivetr Recovery Codes",
      "====================",
      "",
      "Keep these codes in a safe place. Each code can only be used once.",
      "",
      ...recoveryCodes.map((code, i) => `${i + 1}. ${code}`),
      "",
      `Generated: ${new Date().toISOString()}`,
    ].join("\n");

    const blob = new Blob([content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "rivetr-recovery-codes.txt";
    a.click();
    URL.revokeObjectURL(url);
    toast.success("Recovery codes downloaded");
  }

  function handleFinishSetup() {
    setSetupStep("idle");
    setSetupData(null);
    setVerifyCode("");
    setRecoveryCodes([]);
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">Security</h1>
        <Card>
          <CardContent className="flex items-center justify-center py-12">
            <Loader2 className="size-6 animate-spin text-muted-foreground" />
          </CardContent>
        </Card>
      </div>
    );
  }

  const is2FAEnabled = status?.enabled ?? false;

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold">Security</h1>

      {/* 2FA Status Card */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                Two-Factor Authentication
                {is2FAEnabled ? (
                  <Badge
                    variant="default"
                    className="bg-green-600 hover:bg-green-700"
                  >
                    Enabled
                  </Badge>
                ) : (
                  <Badge variant="secondary">Disabled</Badge>
                )}
              </CardTitle>
              <CardDescription className="mt-1">
                Add an extra layer of security to your account by requiring a
                verification code from an authenticator app when signing in.
              </CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {is2FAEnabled ? (
            <div className="space-y-4">
              <div className="flex items-start gap-3 p-4 rounded-lg bg-green-50 dark:bg-green-950/20 border border-green-200 dark:border-green-900">
                <CheckCircle2 className="size-5 text-green-600 mt-0.5 shrink-0" />
                <div>
                  <p className="text-sm font-medium text-green-800 dark:text-green-200">
                    Your account is protected with two-factor authentication
                  </p>
                  <p className="text-sm text-green-700 dark:text-green-300 mt-1">
                    You will be asked to enter a verification code from your
                    authenticator app each time you sign in.
                  </p>
                </div>
              </div>
              <Button
                variant="destructive"
                onClick={() => setShowDisableDialog(true)}
              >
                <ShieldOff className="size-4 mr-2" />
                Disable Two-Factor Authentication
              </Button>
            </div>
          ) : setupStep === "idle" ? (
            <div className="space-y-4">
              <p className="text-sm text-muted-foreground">
                Two-factor authentication is not enabled. Enable it to add
                an extra layer of security to your account.
              </p>
              <Button
                onClick={() => setupMutation.mutate()}
                disabled={setupMutation.isPending}
              >
                {setupMutation.isPending ? (
                  <Loader2 className="size-4 mr-2 animate-spin" />
                ) : (
                  <ShieldCheck className="size-4 mr-2" />
                )}
                Enable Two-Factor Authentication
              </Button>
            </div>
          ) : setupStep === "scanning" ? (
            <div className="space-y-6">
              <Alert>
                <ShieldCheck className="size-4" />
                <AlertTitle>Step 1: Scan QR Code</AlertTitle>
                <AlertDescription>
                  Scan the QR code below with your authenticator app (Google
                  Authenticator, Authy, 1Password, etc.).
                </AlertDescription>
              </Alert>

              {setupData && (
                <div className="flex flex-col items-center gap-4">
                  <div className="p-4 bg-white rounded-lg border">
                    <img
                      src={`data:image/png;base64,${setupData.qr_code_svg}`}
                      alt="2FA QR Code"
                      className="size-48"
                    />
                  </div>

                  <div className="text-center space-y-2">
                    <p className="text-sm text-muted-foreground">
                      Or enter this code manually:
                    </p>
                    <code className="block px-4 py-2 bg-muted rounded-md font-mono text-sm break-all select-all">
                      {setupData.secret}
                    </code>
                  </div>
                </div>
              )}

              <div className="flex gap-2">
                <Button
                  onClick={() => setSetupStep("verifying")}
                  className="flex-1"
                >
                  Next: Verify Code
                </Button>
                <Button
                  variant="outline"
                  onClick={() => {
                    setSetupStep("idle");
                    setSetupData(null);
                  }}
                >
                  Cancel
                </Button>
              </div>
            </div>
          ) : setupStep === "verifying" ? (
            <div className="space-y-6">
              <Alert>
                <ShieldCheck className="size-4" />
                <AlertTitle>Step 2: Verify Code</AlertTitle>
                <AlertDescription>
                  Enter the 6-digit code from your authenticator app to verify
                  the setup.
                </AlertDescription>
              </Alert>

              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  if (verifyCode.length === 6) {
                    verifyMutation.mutate(verifyCode);
                  }
                }}
                className="space-y-4"
              >
                <div className="space-y-2">
                  <Label htmlFor="verify-code">Verification Code</Label>
                  <Input
                    id="verify-code"
                    type="text"
                    inputMode="numeric"
                    autoComplete="one-time-code"
                    placeholder="000000"
                    value={verifyCode}
                    onChange={(e) => {
                      const v = e.target.value.replace(/\D/g, "").slice(0, 6);
                      setVerifyCode(v);
                    }}
                    className="text-center text-2xl tracking-[0.5em] font-mono max-w-[240px]"
                    maxLength={6}
                    autoFocus
                  />
                </div>
                <div className="flex gap-2">
                  <Button
                    type="submit"
                    disabled={
                      verifyCode.length !== 6 || verifyMutation.isPending
                    }
                    className="flex-1"
                  >
                    {verifyMutation.isPending ? (
                      <Loader2 className="size-4 mr-2 animate-spin" />
                    ) : null}
                    Verify and Enable
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setSetupStep("scanning")}
                  >
                    Back
                  </Button>
                </div>
              </form>
            </div>
          ) : setupStep === "recovery" ? (
            <div className="space-y-6">
              <Alert variant="destructive">
                <AlertTriangle className="size-4" />
                <AlertTitle>Save Your Recovery Codes</AlertTitle>
                <AlertDescription>
                  These recovery codes can be used to access your account if you
                  lose your authenticator device. Each code can only be used
                  once. Store them in a safe place -- they will not be shown
                  again.
                </AlertDescription>
              </Alert>

              <div className="grid grid-cols-2 gap-2 p-4 bg-muted rounded-lg font-mono text-sm">
                {recoveryCodes.map((code, i) => (
                  <div key={i} className="px-2 py-1">
                    {code}
                  </div>
                ))}
              </div>

              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={handleCopyRecoveryCodes}
                  className="flex-1"
                >
                  <Copy className="size-4 mr-2" />
                  Copy
                </Button>
                <Button
                  variant="outline"
                  onClick={handleDownloadRecoveryCodes}
                  className="flex-1"
                >
                  <Download className="size-4 mr-2" />
                  Download
                </Button>
              </div>

              <Button onClick={handleFinishSetup} className="w-full">
                I have saved my recovery codes
              </Button>
            </div>
          ) : null}
        </CardContent>
      </Card>

      {/* Disable 2FA Dialog */}
      <Dialog open={showDisableDialog} onOpenChange={setShowDisableDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Disable Two-Factor Authentication</DialogTitle>
            <DialogDescription>
              Enter your current TOTP code or your account password to disable
              two-factor authentication.
            </DialogDescription>
          </DialogHeader>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (disableCode.trim()) {
                disableMutation.mutate(disableCode.trim());
              }
            }}
          >
            <div className="space-y-4 py-4">
              <Alert variant="destructive">
                <AlertTriangle className="size-4" />
                <AlertDescription>
                  Disabling 2FA will remove the extra security from your
                  account. You can re-enable it at any time.
                </AlertDescription>
              </Alert>
              <div className="space-y-2">
                <Label htmlFor="disable-code">
                  TOTP Code or Password
                </Label>
                <Input
                  id="disable-code"
                  type="text"
                  placeholder="Enter code or password"
                  value={disableCode}
                  onChange={(e) => setDisableCode(e.target.value)}
                  autoFocus
                />
              </div>
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  setShowDisableDialog(false);
                  setDisableCode("");
                }}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                variant="destructive"
                disabled={!disableCode.trim() || disableMutation.isPending}
              >
                {disableMutation.isPending ? (
                  <Loader2 className="size-4 mr-2 animate-spin" />
                ) : null}
                Disable 2FA
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
