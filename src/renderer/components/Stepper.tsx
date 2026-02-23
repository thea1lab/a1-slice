import type { WizardStep } from '../../shared/types'

const STEPS: { key: WizardStep; label: string }[] = [
  { key: 'select', label: 'Select' },
  { key: 'transcribe', label: 'Transcribe' },
  { key: 'review-transcript', label: 'Review' },
  { key: 'review-slices', label: 'Slices' },
  { key: 'export', label: 'Export' }
]

interface StepperProps {
  currentStep: WizardStep
  completedSteps: Set<WizardStep>
  onStepClick: (step: WizardStep) => void
}

export default function Stepper({
  currentStep,
  completedSteps,
  onStepClick
}: StepperProps): React.JSX.Element {
  const currentIndex = STEPS.findIndex((s) => s.key === currentStep)

  return (
    <div className="flex items-center justify-center gap-0 px-8 py-4 shrink-0">
      {STEPS.map((step, i) => {
        const isCompleted = completedSteps.has(step.key)
        const isCurrent = step.key === currentStep
        const isClickable = isCompleted && !isCurrent
        const isPast = i < currentIndex

        return (
          <div key={step.key} className="flex items-center">
            {/* Connector line (before each step except the first) */}
            {i > 0 && (
              <div
                className={`w-12 h-0.5 transition-colors ${
                  isPast || isCurrent ? 'bg-accent/50' : 'bg-white/10'
                }`}
              />
            )}

            {/* Step circle + label */}
            <button
              onClick={() => isClickable && onStepClick(step.key)}
              disabled={!isClickable}
              className={`flex flex-col items-center gap-1.5 group ${
                isClickable ? 'cursor-pointer' : 'cursor-default'
              }`}
            >
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center text-xs font-semibold transition-all ${
                  isCompleted && !isCurrent
                    ? 'bg-accent text-black'
                    : isCurrent
                      ? 'border-2 border-accent text-accent bg-transparent'
                      : 'border border-white/20 text-neutral-500 bg-transparent'
                } ${isClickable ? 'group-hover:scale-110' : ''}`}
              >
                {isCompleted && !isCurrent ? (
                  <svg
                    viewBox="0 0 16 16"
                    fill="none"
                    className="w-3.5 h-3.5"
                  >
                    <path
                      d="M3.5 8.5L6.5 11.5L12.5 5.5"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                ) : (
                  i + 1
                )}
              </div>
              <span
                className={`text-[11px] font-medium whitespace-nowrap ${
                  isCurrent
                    ? 'text-accent'
                    : isCompleted
                      ? 'text-neutral-400'
                      : 'text-neutral-600'
                }`}
              >
                {step.label}
              </span>
            </button>
          </div>
        )
      })}
    </div>
  )
}
