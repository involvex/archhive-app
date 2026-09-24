import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight, X } from "lucide-react";

interface WizardLayoutProps {
  step: number;
  totalSteps: number;
  title: string;
  subtitle?: string;
  children: ReactNode;
  onBack: () => void;
  onNext: () => void;
  onSkip: () => void;
  nextLabel?: string;
  canNext?: boolean;
  isLast?: boolean;
  isFinished?: boolean;
}

export function WizardLayout({
  step,
  totalSteps,
  title,
  subtitle,
  children,
  onBack,
  onNext,
  onSkip,
  nextLabel = "Next",
  canNext = true,
  isLast = false,
  isFinished = false,
}: WizardLayoutProps) {
  return (
    <div className="fixed inset-0 z-[200] flex items-center justify-center bg-black/80 p-4">
      <div className="flex max-h-[92dvh] w-full max-w-2xl flex-col overflow-hidden rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[var(--color-border)] px-6 py-4">
          <div>
            <h2 className="text-lg font-semibold">{title}</h2>
            {subtitle && <p className="text-xs text-[var(--color-muted-foreground)]">{subtitle}</p>}
          </div>
          <button
            type="button"
            onClick={onSkip}
            className="flex min-h-11 min-w-11 items-center justify-center rounded hover:bg-[var(--color-muted)]"
            aria-label="Skip wizard"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Progress */}
        <div className="flex items-center gap-1 border-b border-[var(--color-border)] px-6 py-2">
          {Array.from({ length: totalSteps }, (_, i) => (
            <div key={`wizard-step-${i}-${totalSteps}`} className="flex items-center gap-1">
              <div
                className={`h-2 w-8 rounded-full transition-colors ${
                  i < step
                    ? "bg-[var(--color-primary)]"
                    : i === step
                      ? "bg-[var(--color-primary)]/50"
                      : "bg-[var(--color-muted)]"
                }`}
              />
              {i < totalSteps - 1 && <div className="h-px w-4 bg-[var(--color-border)]" />}
            </div>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          <p className="mb-1 text-xs font-medium text-[var(--color-muted-foreground)]">
            Step {step + 1} of {totalSteps}
          </p>
          {children}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-[var(--color-border)] px-6 py-4">
          <Button variant="ghost" size="sm" onClick={onBack} disabled={step === 0 || isFinished}>
            <ChevronLeft className="mr-1 h-4 w-4" />
            Back
          </Button>
          <span className="text-xs text-[var(--color-muted-foreground)]">
            {isFinished ? "All done!" : `${step + 1} / ${totalSteps}`}
          </span>
          <Button variant="default" size="sm" onClick={onNext} disabled={!canNext || isFinished}>
            {isFinished ? "Finish" : isLast ? nextLabel : nextLabel}
            {!isFinished && <ChevronRight className="ml-1 h-4 w-4" />}
          </Button>
        </div>
      </div>
    </div>
  );
}
