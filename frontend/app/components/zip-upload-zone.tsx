import { useState, useCallback, useRef } from "react";
import { Upload, FileArchive, X, Loader2, CheckCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { BuildDetectionResult } from "@/types/api";

interface ZipUploadZoneProps {
  /** Called when a file is selected */
  onFileSelect: (file: File) => void;
  /** Called when build type detection completes */
  onDetectionComplete?: (result: BuildDetectionResult) => void;
  /** Whether an upload is in progress */
  isUploading?: boolean;
  /** Maximum file size in MB (default: 100) */
  maxSizeMB?: number;
  /** Detection result to display */
  detectionResult?: BuildDetectionResult | null;
  /** Whether to show the detection result */
  showDetection?: boolean;
  /** Custom class name */
  className?: string;
  /** Disabled state */
  disabled?: boolean;
}

export function ZipUploadZone({
  onFileSelect,
  onDetectionComplete,
  isUploading = false,
  maxSizeMB = 100,
  detectionResult,
  showDetection = true,
  className,
  disabled = false,
}: ZipUploadZoneProps) {
  const [isDragOver, setIsDragOver] = useState(false);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const maxSizeBytes = maxSizeMB * 1024 * 1024;

  const validateFile = useCallback(
    (file: File): string | null => {
      // Check file type
      if (!file.name.toLowerCase().endsWith(".zip")) {
        return "Only ZIP files are supported";
      }

      // Check file size
      if (file.size > maxSizeBytes) {
        return `File too large. Maximum size is ${maxSizeMB}MB`;
      }

      return null;
    },
    [maxSizeBytes, maxSizeMB]
  );

  const handleFile = useCallback(
    (file: File) => {
      const validationError = validateFile(file);
      if (validationError) {
        setError(validationError);
        setSelectedFile(null);
        return;
      }

      setError(null);
      setSelectedFile(file);
      onFileSelect(file);
    },
    [validateFile, onFileSelect]
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragOver(false);

      if (disabled || isUploading) return;

      const files = e.dataTransfer.files;
      if (files.length > 0) {
        handleFile(files[0]);
      }
    },
    [disabled, isUploading, handleFile]
  );

  const handleFileInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files && files.length > 0) {
        handleFile(files[0]);
      }
    },
    [handleFile]
  );

  const handleClick = useCallback(() => {
    if (!disabled && !isUploading) {
      fileInputRef.current?.click();
    }
  }, [disabled, isUploading]);

  const handleClear = useCallback(() => {
    setSelectedFile(null);
    setError(null);
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  }, []);

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getBuildTypeLabel = (buildType: string): string => {
    const labels: Record<string, string> = {
      dockerfile: "Dockerfile",
      nixpacks: "Nixpacks",
      static: "Static Site",
      docker_compose: "Docker Compose",
      docker_image: "Docker Image",
    };
    return labels[buildType] || buildType;
  };

  const getConfidenceColor = (confidence: number): string => {
    if (confidence >= 0.8) return "text-green-600 dark:text-green-400";
    if (confidence >= 0.5) return "text-yellow-600 dark:text-yellow-400";
    return "text-red-600 dark:text-red-400";
  };

  return (
    <div className={cn("space-y-4", className)}>
      {/* Drop Zone */}
      <div
        onClick={handleClick}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        className={cn(
          "relative border-2 border-dashed rounded-lg p-8 text-center transition-all cursor-pointer",
          "hover:border-primary/50 hover:bg-muted/50",
          isDragOver && "border-primary bg-primary/5",
          error && "border-destructive",
          (disabled || isUploading) && "opacity-50 cursor-not-allowed",
          selectedFile && !error && "border-primary/30 bg-primary/5"
        )}
      >
        <input
          ref={fileInputRef}
          type="file"
          accept=".zip"
          onChange={handleFileInput}
          className="hidden"
          disabled={disabled || isUploading}
        />

        {isUploading ? (
          <div className="flex flex-col items-center gap-3">
            <Loader2 className="h-12 w-12 text-primary animate-spin" />
            <div>
              <p className="text-lg font-medium">Uploading...</p>
              <p className="text-sm text-muted-foreground">
                Processing your project files
              </p>
            </div>
          </div>
        ) : selectedFile ? (
          <div className="flex flex-col items-center gap-3">
            <FileArchive className="h-12 w-12 text-primary" />
            <div>
              <p className="text-lg font-medium">{selectedFile.name}</p>
              <p className="text-sm text-muted-foreground">
                {formatFileSize(selectedFile.size)}
              </p>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={(e) => {
                e.stopPropagation();
                handleClear();
              }}
              className="text-muted-foreground hover:text-foreground"
            >
              <X className="h-4 w-4 mr-1" />
              Clear
            </Button>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-3">
            <Upload className="h-12 w-12 text-muted-foreground" />
            <div>
              <p className="text-lg font-medium">
                Drop your ZIP file here or click to browse
              </p>
              <p className="text-sm text-muted-foreground">
                Maximum file size: {maxSizeMB}MB
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Error Message */}
      {error && (
        <p className="text-sm text-destructive flex items-center gap-1">
          <X className="h-4 w-4" />
          {error}
        </p>
      )}

      {/* Detection Result */}
      {showDetection && detectionResult && (
        <Card className="p-4">
          <div className="flex items-start gap-4">
            <CheckCircle className="h-5 w-5 text-green-500 mt-0.5" />
            <div className="flex-1 space-y-2">
              <div className="flex items-center gap-2">
                <span className="font-medium">Build Type Detected:</span>
                <Badge variant="secondary">
                  {getBuildTypeLabel(detectionResult.build_type)}
                </Badge>
                <span
                  className={cn(
                    "text-sm",
                    getConfidenceColor(detectionResult.confidence)
                  )}
                >
                  ({Math.round(detectionResult.confidence * 100)}% confidence)
                </span>
              </div>

              <p className="text-sm text-muted-foreground">
                Detected from: {detectionResult.detected_from}
              </p>

              {detectionResult.framework && (
                <p className="text-sm">
                  <span className="text-muted-foreground">Framework:</span>{" "}
                  {detectionResult.framework}
                </p>
              )}

              {detectionResult.publish_directory && (
                <p className="text-sm">
                  <span className="text-muted-foreground">Publish directory:</span>{" "}
                  <code className="bg-muted px-1 py-0.5 rounded text-xs">
                    {detectionResult.publish_directory}
                  </code>
                </p>
              )}

              {detectionResult.language && (
                <p className="text-sm">
                  <span className="text-muted-foreground">Language:</span>{" "}
                  {detectionResult.language}
                </p>
              )}
            </div>
          </div>
        </Card>
      )}
    </div>
  );
}
