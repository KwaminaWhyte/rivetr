import { useState } from "react";
import { useOutletContext, Form, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import type { Service } from "@/types/api";
import { Trash2, AlertTriangle } from "lucide-react";

interface OutletContext {
  service: Service;
  token: string;
}

export default function ServiceSettingsTab() {
  const { service } = useOutletContext<OutletContext>();
  const navigation = useNavigation();
  const [deleteConfirmName, setDeleteConfirmName] = useState("");

  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="space-y-6">
      {/* Danger Zone */}
      <Card className="border-destructive">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="h-5 w-5" />
            Danger Zone
          </CardTitle>
          <CardDescription>
            Irreversible and destructive actions
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between p-4 border border-destructive/50 rounded-lg">
            <div>
              <h4 className="font-medium">Delete Service</h4>
              <p className="text-sm text-muted-foreground">
                Permanently delete this service and all its data. This action cannot be undone.
              </p>
            </div>
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" className="gap-2">
                  <Trash2 className="h-4 w-4" />
                  Delete Service
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Delete Service</AlertDialogTitle>
                  <AlertDialogDescription>
                    This action cannot be undone. This will permanently delete the service
                    <strong className="text-foreground"> {service.name}</strong> and remove all
                    associated containers and volumes.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <div className="py-4">
                  <Label htmlFor="confirm-name" className="text-sm">
                    Type <strong>{service.name}</strong> to confirm:
                  </Label>
                  <Input
                    id="confirm-name"
                    value={deleteConfirmName}
                    onChange={(e) => setDeleteConfirmName(e.target.value)}
                    placeholder={service.name}
                    className="mt-2"
                  />
                </div>
                <AlertDialogFooter>
                  <AlertDialogCancel onClick={() => setDeleteConfirmName("")}>
                    Cancel
                  </AlertDialogCancel>
                  <Form method="post">
                    <input type="hidden" name="intent" value="delete" />
                    <input type="hidden" name="projectId" value={service.project_id || ""} />
                    <AlertDialogAction
                      type="submit"
                      disabled={deleteConfirmName !== service.name || isSubmitting}
                      className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    >
                      {isSubmitting ? "Deleting..." : "Delete Service"}
                    </AlertDialogAction>
                  </Form>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
