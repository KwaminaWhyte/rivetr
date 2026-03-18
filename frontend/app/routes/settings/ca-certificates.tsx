import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { ShieldCheck, Plus, Trash2, Loader2 } from "lucide-react";
import { caCertificatesApi } from "@/lib/api/ca-certificates";
import type { CaCertificate } from "@/lib/api/ca-certificates";

export function meta() {
  return [
    { title: "CA Certificates - Rivetr" },
    { name: "description", content: "Manage custom CA certificates for private certificate authorities" },
  ];
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString();
}

function truncateCert(cert: string): string {
  const lines = cert.trim().split("\n");
  if (lines.length <= 3) return cert;
  return `${lines[0]}\n${lines[1]}\n...\n${lines[lines.length - 1]}`;
}

export default function CaCertificatesPage() {
  const queryClient = useQueryClient();
  const [addDialogOpen, setAddDialogOpen] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [formName, setFormName] = useState("");
  const [formCert, setFormCert] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { data: certs = [], isLoading } = useQuery<CaCertificate[]>({
    queryKey: ["ca-certificates"],
    queryFn: () => caCertificatesApi.list(),
  });

  const createMutation = useMutation({
    mutationFn: () =>
      caCertificatesApi.create({ name: formName.trim(), certificate: formCert.trim() }),
    onSuccess: () => {
      toast.success("CA certificate added");
      queryClient.invalidateQueries({ queryKey: ["ca-certificates"] });
      resetForm();
      setAddDialogOpen(false);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to add CA certificate");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => caCertificatesApi.delete(id),
    onSuccess: () => {
      toast.success("CA certificate deleted");
      queryClient.invalidateQueries({ queryKey: ["ca-certificates"] });
      setDeleteId(null);
    },
    onError: (error: Error) => {
      toast.error(error.message || "Failed to delete CA certificate");
    },
  });

  const resetForm = () => {
    setFormName("");
    setFormCert("");
  };

  const handleAdd = async () => {
    if (!formName.trim()) {
      toast.error("Name is required");
      return;
    }
    if (!formCert.trim()) {
      toast.error("Certificate PEM is required");
      return;
    }
    setIsSubmitting(true);
    try {
      await createMutation.mutateAsync();
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">CA Certificates</h1>
        <p className="text-muted-foreground">
          Store custom CA certificates for servers using private certificate authorities.
        </p>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <ShieldCheck className="h-5 w-5" />
                Certificates
              </CardTitle>
              <CardDescription>
                PEM-format CA certificates trusted by this Rivetr instance.
              </CardDescription>
            </div>
            <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
              <Plus className="h-4 w-4" />
              Add Certificate
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : certs.length === 0 ? (
            <div className="text-center py-12 space-y-4">
              <ShieldCheck className="h-12 w-12 mx-auto text-muted-foreground/50" />
              <div>
                <p className="text-lg font-medium">No CA Certificates</p>
                <p className="text-sm text-muted-foreground">
                  Add a CA certificate to trust private certificate authorities.
                </p>
              </div>
              <Button onClick={() => setAddDialogOpen(true)} className="gap-2">
                <Plus className="h-4 w-4" />
                Add Certificate
              </Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Certificate (preview)</TableHead>
                  <TableHead>Added</TableHead>
                  <TableHead className="w-[80px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {certs.map((cert) => (
                  <TableRow key={cert.id}>
                    <TableCell className="font-medium">{cert.name}</TableCell>
                    <TableCell>
                      <pre className="text-xs font-mono text-muted-foreground max-w-sm truncate">
                        {truncateCert(cert.certificate)}
                      </pre>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatDate(cert.created_at)}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setDeleteId(cert.id)}
                        className="text-destructive hover:text-destructive"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Add Certificate Dialog */}
      <Dialog
        open={addDialogOpen}
        onOpenChange={(open) => {
          setAddDialogOpen(open);
          if (!open) resetForm();
        }}
      >
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5" />
              Add CA Certificate
            </DialogTitle>
            <DialogDescription>
              Paste a PEM-encoded certificate. It must start with{" "}
              <code className="font-mono text-xs">-----BEGIN CERTIFICATE-----</code>.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="cert-name">Name</Label>
              <Input
                id="cert-name"
                placeholder="My Private CA"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="cert-pem">Certificate (PEM)</Label>
              <Textarea
                id="cert-pem"
                placeholder={"-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----"}
                value={formCert}
                onChange={(e) => setFormCert(e.target.value)}
                rows={8}
                className="font-mono text-xs"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddDialogOpen(false)}>
              Cancel
            </Button>
            <Button onClick={handleAdd} disabled={isSubmitting} className="gap-2">
              {isSubmitting ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Plus className="h-4 w-4" />
              )}
              Add Certificate
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <AlertDialog open={!!deleteId} onOpenChange={() => setDeleteId(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Certificate</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete this CA certificate? This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (deleteId) deleteMutation.mutate(deleteId);
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
