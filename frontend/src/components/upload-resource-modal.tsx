import { useState, useRef, useCallback } from "react";
import { Loader2, Upload, FileUp } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import { uploadResource, createTopic } from "@/lib/api";
import type { Topic, Resource } from "@/types/study";

const ACCEPTED = ".pdf,.txt,.md";

interface UploadResourceModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  topics: Topic[];
  defaultTopicId?: string;
  onUploaded: (resource: Resource) => void;
  onTopicCreated: (topic: Topic) => void;
}

export function UploadResourceModal({
  open,
  onOpenChange,
  topics,
  defaultTopicId,
  onUploaded,
  onTopicCreated,
}: UploadResourceModalProps) {
  const [topicId, setTopicId] = useState<string>(defaultTopicId ?? "");
  const [newTopicName, setNewTopicName] = useState("");
  const [isNewTopic, setIsNewTopic] = useState(false);
  const [file, setFile] = useState<File | null>(null);
  const [title, setTitle] = useState("");
  const [uploading, setUploading] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [dragOver, setDragOver] = useState(false);

  const reset = () => {
    setTopicId(defaultTopicId ?? "");
    setNewTopicName("");
    setIsNewTopic(false);
    setFile(null);
    setTitle("");
    setUploading(false);
    setDragOver(false);
  };

  const handleFileSelect = (f: File) => {
    setFile(f);
    if (!title) {
      setTitle(f.name.replace(/\.[^.]+$/, ""));
    }
  };

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const f = e.dataTransfer.files[0];
    if (f) handleFileSelect(f);
  }, [title]);

  const handleTopicChange = (value: string) => {
    if (value === "__new__") {
      setIsNewTopic(true);
      setTopicId("");
    } else {
      setIsNewTopic(false);
      setTopicId(value);
    }
  };

  const canSubmit =
    file &&
    title.trim() &&
    (topicId || (isNewTopic && newTopicName.trim())) &&
    !uploading;

  const handleUpload = async () => {
    if (!canSubmit || !file) return;

    setUploading(true);
    try {
      let resolvedTopicId = topicId;

      if (isNewTopic) {
        const topic = await createTopic({ name: newTopicName.trim() });
        resolvedTopicId = topic.id;
        onTopicCreated(topic);
      }

      const resource = await uploadResource(file, resolvedTopicId, title.trim(), []);
      toast.success("Resource uploaded successfully");
      onUploaded(resource);
      onOpenChange(false);
      reset();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Upload failed");
    } finally {
      setUploading(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(v) => {
        if (!v) reset();
        onOpenChange(v);
      }}
    >
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="font-display flex items-center gap-2">
            <Upload className="h-5 w-5 text-accent" />
            Upload Resource
          </DialogTitle>
          <DialogDescription>
            Upload a PDF, text, or markdown file to start studying.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Topic */}
          <div className="space-y-2">
            <Label>Topic</Label>
            <Select
              value={isNewTopic ? "__new__" : topicId}
              onValueChange={handleTopicChange}
            >
              <SelectTrigger>
                <SelectValue placeholder="Select a topic" />
              </SelectTrigger>
              <SelectContent>
                {topics.map((t) => (
                  <SelectItem key={t.id} value={t.id}>
                    {t.name}
                  </SelectItem>
                ))}
                <SelectItem value="__new__">+ Create new topic</SelectItem>
              </SelectContent>
            </Select>
            {isNewTopic && (
              <Input
                autoFocus
                placeholder="New topic name"
                value={newTopicName}
                onChange={(e) => setNewTopicName(e.target.value)}
              />
            )}
          </div>

          {/* File drop zone */}
          <div className="space-y-2">
            <Label>File</Label>
            <div
              onDragOver={(e) => {
                e.preventDefault();
                setDragOver(true);
              }}
              onDragLeave={() => setDragOver(false)}
              onDrop={handleDrop}
              onClick={() => fileInputRef.current?.click()}
              className={`
                flex flex-col items-center justify-center gap-2 rounded-lg border-2 border-dashed p-6 cursor-pointer transition-colors
                ${dragOver ? "border-accent bg-accent/5" : "border-muted-foreground/25 hover:border-muted-foreground/50"}
              `}
            >
              <FileUp className="h-8 w-8 text-muted-foreground" />
              {file ? (
                <p className="text-sm font-medium text-foreground">{file.name}</p>
              ) : (
                <p className="text-sm text-muted-foreground">
                  Drop a file here or click to browse
                </p>
              )}
              <p className="text-xs text-muted-foreground">PDF, TXT, or Markdown</p>
              <input
                ref={fileInputRef}
                type="file"
                accept={ACCEPTED}
                className="hidden"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  if (f) handleFileSelect(f);
                }}
              />
            </div>
          </div>

          {/* Title */}
          <div className="space-y-2">
            <Label>Title</Label>
            <Input
              placeholder="Resource title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </div>
        </div>

        <DialogFooter>
          <Button onClick={handleUpload} disabled={!canSubmit}>
            {uploading ? (
              <Loader2 className="h-4 w-4 animate-spin mr-1.5" />
            ) : (
              <Upload className="h-4 w-4 mr-1.5" />
            )}
            {uploading ? "Uploading..." : "Upload"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
