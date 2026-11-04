import { useNavigate } from "react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
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
import { api } from "@/lib/api";
import type { Service, ServiceDeployment } from "@/types/api";
import { Undo2 } from "lucide-react";

/** Redeploy the exact images (by digest) a previous deployment ran. */
export function ServiceRollbackButton({
  service,
  deployment,
}: {
  service: Service;
  deployment: ServiceDeployment;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: () => api.rollbackServiceDeployment(service.id, deployment.id),
    onSuccess: (dep) => {
      toast.success("Rollback started");
      queryClient.invalidateQueries({ queryKey: ["service-deployments", service.id] });
      navigate(`/services/${service.id}/deployments?deployment=${dep.id}`);
    },
    onError: (e) => toast.error(e instanceof Error ? e.message : "Rollback failed to start"),
  });

  const images = Object.entries(deployment.images_after ?? {});

  return (
    <AlertDialog>
      <AlertDialogTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2" disabled={mutation.isPending}>
          <Undo2 className="h-4 w-4" />
          Roll back to this
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Roll back {service.name}?</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-2">
              <p>
                Runs the images this deployment ran, pinned by digest. No build and no hooks run;
                database migrations are not reversed. The pins stay until the next Deploy, which
                goes back to the tags in the compose file.
              </p>
              <ul className="space-y-0.5 font-mono text-xs">
                {images.map(([name, s]) => (
                  <li key={name} className="break-all">
                    {name}: {s.digest ?? s.image_id ?? s.image}
                  </li>
                ))}
              </ul>
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction onClick={() => mutation.mutate()}>Roll back</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
