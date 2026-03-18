import { useState, useEffect } from "react";
import { Moon, Sun, Monitor, SlidersHorizontal } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useTheme } from "@/components/providers/theme-provider";

export function meta() {
  return [
    { title: "Preferences - Rivetr" },
    { name: "description", content: "Manage your personal UI preferences" },
  ];
}

const PREF_DATE_FORMAT = "pref_date_format";
const PREF_LOG_LINES = "pref_log_lines";
const PREF_DEPLOY_NOTIFY = "pref_deploy_notify";
const PREF_COMPACT_MODE = "pref_compact_mode";

function readPref<T>(key: string, fallback: T): T {
  if (typeof window === "undefined") return fallback;
  const stored = localStorage.getItem(key);
  if (stored === null) return fallback;
  try {
    return JSON.parse(stored) as T;
  } catch {
    return fallback;
  }
}

function writePref(key: string, value: unknown) {
  if (typeof window === "undefined") return;
  localStorage.setItem(key, JSON.stringify(value));
}

export default function SettingsPreferencesPage() {
  const { theme, setTheme } = useTheme();

  const [dateFormat, setDateFormat] = useState<"relative" | "absolute">(() =>
    readPref(PREF_DATE_FORMAT, "relative")
  );
  const [logLines, setLogLines] = useState<string>(() =>
    readPref(PREF_LOG_LINES, "100")
  );
  const [deployNotify, setDeployNotify] = useState<boolean>(() =>
    readPref(PREF_DEPLOY_NOTIFY, false)
  );
  const [compactMode, setCompactMode] = useState<boolean>(() =>
    readPref(PREF_COMPACT_MODE, false)
  );

  // Apply compact mode to root element
  useEffect(() => {
    const root = document.documentElement;
    if (compactMode) {
      root.classList.add("compact");
    } else {
      root.classList.remove("compact");
    }
  }, [compactMode]);

  const handleDateFormatChange = (value: "relative" | "absolute") => {
    setDateFormat(value);
    writePref(PREF_DATE_FORMAT, value);
  };

  const handleLogLinesChange = (value: string) => {
    setLogLines(value);
    writePref(PREF_LOG_LINES, value);
  };

  const handleDeployNotifyChange = (checked: boolean) => {
    setDeployNotify(checked);
    writePref(PREF_DEPLOY_NOTIFY, checked);
  };

  const handleCompactModeChange = (checked: boolean) => {
    setCompactMode(checked);
    writePref(PREF_COMPACT_MODE, checked);
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold flex items-center gap-2">
          <SlidersHorizontal className="h-7 w-7" />
          Preferences
        </h1>
        <p className="text-muted-foreground mt-1">
          Customize your personal dashboard experience. Changes are saved
          instantly and stored locally in your browser.
        </p>
      </div>

      {/* Theme */}
      <Card>
        <CardHeader>
          <CardTitle>Appearance</CardTitle>
          <CardDescription>
            Choose how the dashboard looks. "System" follows your OS setting.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <Label>Theme</Label>
            <div className="flex gap-2 flex-wrap">
              <Button
                variant={theme === "light" ? "default" : "outline"}
                size="sm"
                className="gap-2"
                onClick={() => setTheme("light")}
              >
                <Sun className="h-4 w-4" />
                Light
              </Button>
              <Button
                variant={theme === "dark" ? "default" : "outline"}
                size="sm"
                className="gap-2"
                onClick={() => setTheme("dark")}
              >
                <Moon className="h-4 w-4" />
                Dark
              </Button>
              <Button
                variant={theme === "system" ? "default" : "outline"}
                size="sm"
                className="gap-2"
                onClick={() => setTheme("system")}
              >
                <Monitor className="h-4 w-4" />
                System
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Currently active: <span className="font-medium">{theme}</span>
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Date/time format */}
      <Card>
        <CardHeader>
          <CardTitle>Date &amp; Time Display</CardTitle>
          <CardDescription>
            Control how timestamps appear throughout the dashboard.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <Label htmlFor="date-format">Date format</Label>
            <Select value={dateFormat} onValueChange={handleDateFormatChange}>
              <SelectTrigger id="date-format" className="w-56">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="relative">
                  Relative — "2 hours ago"
                </SelectItem>
                <SelectItem value="absolute">
                  Absolute — "Mar 18, 2026 5:30 PM"
                </SelectItem>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              Applies to deployment timestamps, log entries, and activity feeds.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Default log lines */}
      <Card>
        <CardHeader>
          <CardTitle>Log Viewer</CardTitle>
          <CardDescription>
            Configure how many log lines are fetched by default when opening the
            log viewer.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <Label htmlFor="log-lines">Default log lines</Label>
            <Select value={logLines} onValueChange={handleLogLinesChange}>
              <SelectTrigger id="log-lines" className="w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="100">100 lines</SelectItem>
                <SelectItem value="500">500 lines</SelectItem>
                <SelectItem value="1000">1000 lines</SelectItem>
                <SelectItem value="all">All lines</SelectItem>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              Fetching more lines may slow page load for verbose applications.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Deployment notifications */}
      <Card>
        <CardHeader>
          <CardTitle>Notifications</CardTitle>
          <CardDescription>
            Control how and when you receive in-browser alerts.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="deploy-notify">Deployment notifications</Label>
              <p className="text-xs text-muted-foreground">
                Receive a browser notification when a deployment finishes (success
                or failure). Requires browser notification permission when enabled.
              </p>
            </div>
            <Switch
              id="deploy-notify"
              checked={deployNotify}
              onCheckedChange={handleDeployNotifyChange}
            />
          </div>
        </CardContent>
      </Card>

      {/* Compact mode */}
      <Card>
        <CardHeader>
          <CardTitle>Layout Density</CardTitle>
          <CardDescription>
            Reduce padding and spacing throughout the UI for a denser layout.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="compact-mode">Compact mode</Label>
              <p className="text-xs text-muted-foreground">
                Adds a <code className="bg-muted px-1 rounded font-mono text-xs">compact</code> class
                to the root element. Useful on smaller screens or for power users
                who prefer less whitespace.
              </p>
            </div>
            <Switch
              id="compact-mode"
              checked={compactMode}
              onCheckedChange={handleCompactModeChange}
            />
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
