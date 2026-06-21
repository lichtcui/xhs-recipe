import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { AlertCircle, XCircle } from "lucide-react";
import type { ReactNode } from "react";

interface ErrorAlertProps {
  title?: string;
  message: string;
  variant?: "destructive" | "warning";
  action?: ReactNode;
}

export default function ErrorAlert({
  title,
  message,
  variant = "destructive",
  action,
}: ErrorAlertProps) {
  const Icon = variant === "destructive" ? XCircle : AlertCircle;
  const bgClass =
    variant === "destructive"
      ? "border-red-200 bg-red-50"
      : "border-amber-200 bg-amber-50";
  const textClass =
    variant === "destructive" ? "text-red-800" : "text-amber-800";

  return (
    <Alert className={bgClass}>
      <Icon className={`h-4 w-4 ${textClass}`} />
      {title && <AlertTitle className={textClass}>{title}</AlertTitle>}
      <AlertDescription className={`text-sm ${textClass}`}>
        {message}
      </AlertDescription>
      {action && <div className="mt-2">{action}</div>}
    </Alert>
  );
}
