import { useState, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Plus, Trash2 } from "lucide-react";

export interface LabelEntry {
  key: string;
  value: string;
}

interface ContainerLabelsEditorProps {
  /** Initial labels to populate the editor */
  labels: LabelEntry[];
  /** Called whenever the labels change */
  onChange: (labels: LabelEntry[]) => void;
}

/**
 * A key-value pair editor for custom Docker container labels.
 * Labels are stored in array format: [{key, value}, ...].
 * Rows with an empty key are ignored when saved.
 */
export function ContainerLabelsEditor({
  labels,
  onChange,
}: ContainerLabelsEditorProps) {
  const [rows, setRows] = useState<LabelEntry[]>(labels);

  // Sync external label changes into the editor
  useEffect(() => {
    setRows(labels);
  }, [labels]);

  const update = useCallback(
    (next: LabelEntry[]) => {
      setRows(next);
      onChange(next);
    },
    [onChange]
  );

  const addRow = () => {
    update([...rows, { key: "", value: "" }]);
  };

  const removeRow = (index: number) => {
    update(rows.filter((_, i) => i !== index));
  };

  const updateKey = (index: number, key: string) => {
    const next = rows.map((r, i) => (i === index ? { ...r, key } : r));
    update(next);
  };

  const updateValue = (index: number, value: string) => {
    const next = rows.map((r, i) => (i === index ? { ...r, value } : r));
    update(next);
  };

  return (
    <div className="space-y-3">
      {rows.length > 0 && (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[45%]">Key</TableHead>
              <TableHead>Value</TableHead>
              <TableHead className="w-[52px]" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row, index) => (
              <TableRow key={index}>
                <TableCell className="py-1 pr-2">
                  <Input
                    placeholder="com.example.label"
                    value={row.key}
                    onChange={(e) => updateKey(index, e.target.value)}
                    className="font-mono text-sm h-8"
                  />
                </TableCell>
                <TableCell className="py-1 pr-2">
                  <Input
                    placeholder="value"
                    value={row.value}
                    onChange={(e) => updateValue(index, e.target.value)}
                    className="font-mono text-sm h-8"
                  />
                </TableCell>
                <TableCell className="py-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => removeRow(index)}
                    className="h-8 w-8 p-0 text-red-500 hover:text-red-600"
                    aria-label="Remove label"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}

      {rows.length === 0 && (
        <div className="text-sm text-muted-foreground py-4 text-center border rounded-md">
          No custom labels. Click &ldquo;Add Label&rdquo; to add one.
        </div>
      )}

      <Button variant="outline" size="sm" onClick={addRow} className="gap-2">
        <Plus className="h-4 w-4" />
        Add Label
      </Button>
    </div>
  );
}
