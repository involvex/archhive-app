import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "@/components/ui/button";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/** Prevents a single render crash from blanking the whole Android WebView. */
export class AppErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("AppErrorBoundary", error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex min-h-screen flex-col items-center justify-center gap-4 bg-[var(--color-background)] p-6 text-[var(--color-foreground)]">
          <h1 className="text-lg font-semibold">Something went wrong</h1>
          <p className="max-w-md text-center text-sm text-[var(--color-muted-foreground)]">
            {this.state.error.message || "Unexpected error"}
          </p>
          <Button
            onClick={() => {
              this.setState({ error: null });
              window.location.assign("/");
            }}
          >
            Reload app
          </Button>
        </div>
      );
    }
    return this.props.children;
  }
}
