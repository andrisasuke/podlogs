import { AlertTriangle, RefreshCw, X } from 'lucide-react';
import { Button } from './Button';
import { Modal } from './Modal';

interface ErrorDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onRetry?: () => void;
  title?: string;
  message: string;
  details?: string;
}

export function ErrorDialog({
  isOpen,
  onClose,
  onRetry,
  title = 'Connection Error',
  message,
  details,
}: ErrorDialogProps) {
  return (
    <Modal isOpen={isOpen} onClose={onClose} title={title} size="md" persistent>
      <div className="p-6">
        <div className="flex items-start gap-4">
          <div className="p-3 rounded-full bg-red-500/10">
            <AlertTriangle className="w-6 h-6 text-red-500" />
          </div>
          <div className="flex-1">
            <p className="text-text-primary font-medium mb-2">{message}</p>
            {details && (
              <div className="mt-3 p-3 rounded-md bg-bg-tertiary border border-border">
                <p className="text-xs font-mono text-text-muted break-all whitespace-pre-wrap">
                  {details}
                </p>
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 mt-6 pt-4 border-t border-border">
          <Button variant="secondary" onClick={onClose}>
            <X className="w-4 h-4 mr-1" />
            Dismiss
          </Button>
          {onRetry && (
            <Button variant="primary" onClick={onRetry}>
              <RefreshCw className="w-4 h-4 mr-1" />
              Retry
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}

interface ConnectionErrorProps {
  error: Error | null;
  onDismiss: () => void;
  onRetry?: () => void;
}

export function ConnectionError({ error, onDismiss, onRetry }: ConnectionErrorProps) {
  if (!error) return null;

  const errorMessage = error.message || 'Unknown error occurred';

  // Parse common K8s errors for user-friendly messages
  let userMessage = 'Failed to connect to Kubernetes cluster';
  let details = errorMessage;

  if (errorMessage.includes('No kubeconfig found')) {
    userMessage = 'Kubernetes configuration not found';
    details = 'Please ensure kubectl is configured and ~/.kube/config exists.';
  } else if (errorMessage.includes('connection refused') || errorMessage.includes('ECONNREFUSED')) {
    userMessage = 'Cannot reach Kubernetes cluster';
    details = 'The cluster may be offline or unreachable. Check your network connection and cluster status.';
  } else if (errorMessage.includes('Unauthorized') || errorMessage.includes('401')) {
    userMessage = 'Authentication failed';
    details = 'Your Kubernetes credentials may have expired. Try running: kubectl get pods';
  } else if (errorMessage.includes('Forbidden') || errorMessage.includes('403')) {
    userMessage = 'Access denied';
    details = 'You do not have permission to access this resource. Check your RBAC settings.';
  } else if (errorMessage.includes('timeout') || errorMessage.includes('ETIMEDOUT')) {
    userMessage = 'Connection timed out';
    details = 'The cluster took too long to respond. Check your network connection.';
  } else if (errorMessage.includes('certificate')) {
    userMessage = 'Certificate error';
    details = 'There was a problem with the cluster certificate. ' + errorMessage;
  }

  return (
    <ErrorDialog
      isOpen={true}
      onClose={onDismiss}
      onRetry={onRetry}
      title="Connection Error"
      message={userMessage}
      details={details}
    />
  );
}
