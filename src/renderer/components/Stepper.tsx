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
    <div className="flex items-center justify-center gap-0">
      {STEPS.map((step, i) => {
        const isCompleted = completedSteps.has(step.key)
        const isCurrent = step.key === currentStep
        const isClickable = isCompleted && !isCurrent
        const isPast = i < currentIndex

        return (
          <div key={step.key} className="flex items-center">
            {i > 0 && (
              <div
                className={`w-6 sm:w-10 h-px ${
                  isPast || isCurrent ? 'bg-accent/35' : 'bg-white/10'
                }`}
              />
            )}
            <button
              onClick={() => isClickable && onStepClick(step.key)}
              disabled={!isClickable}
              aria-current={isCurrent ? 'step' : undefined}
              className={`flex items-center gap-1.5 px-1 ${
                isClickable ? 'cursor-pointer' : 'cursor-default'
              }`}
            >
              <span
                className={`w-5 h-5 rounded-full grid place-items-center text-[10px] font-medium ${
                  isCompleted && !isCurrent
                    ? 'bg-accent text-black'
                    : isCurrent
                      ? 'border border-accent text-accent'
                      : 'border border-white/15 text-neutral-500'
                }`}
              >
                {isCompleted && !isCurrent ? (
                  <svg viewBox="0 0 16 16" fill="none" className="w-3 h-3">
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
              </span>
              <span
                className={`text-[11px] whitespace-nowrap ${
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
