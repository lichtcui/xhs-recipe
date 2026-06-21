import { useState } from "react";
import { motion } from "framer-motion";
import { cn } from "@/lib/utils";
import type { Step } from "@/types/recipe";

interface StepTimelineProps {
  steps: Step[];
}

export default function StepTimeline({ steps }: StepTimelineProps) {
  const [currentStep, setCurrentStep] = useState<number | null>(null);

  if (steps.length === 0) return null;

  return (
    <div className="space-y-0">
      {steps.map((step, i) => {
        const isActive = currentStep === i;
        const isLast = i === steps.length - 1;

        return (
          <div key={i} className="relative flex gap-3">
            {/* Timeline line + dot */}
            <div className="flex flex-col items-center shrink-0">
              <motion.button
                onClick={() => setCurrentStep(isActive ? null : i)}
                animate={isActive ? { scale: [1, 1.25, 1] } : { scale: 1 }}
                transition={
                  isActive
                    ? { repeat: Infinity, duration: 1.5 }
                    : { duration: 0.2 }
                }
                className={cn(
                  "w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold shrink-0 border-2 transition-colors",
                  isActive
                    ? "bg-xhs text-white border-xhs shadow-md shadow-xhs/30"
                    : "bg-gray-100 text-gray-400 border-gray-200 hover:border-xhs/50"
                )}
              >
                {i + 1}
              </motion.button>
              {!isLast && (
                <div
                  className={cn(
                    "w-0.5 flex-1 min-h-[24px]",
                    isActive ? "bg-xhs/40" : "bg-gray-200"
                  )}
                />
              )}
            </div>

            {/* Step content */}
            <div
              className={cn(
                "pb-4 flex-1 cursor-pointer",
                isActive && "bg-xhs/5 -mx-2 px-2 rounded-lg"
              )}
              onClick={() => setCurrentStep(isActive ? null : i)}
            >
              <div className="flex items-center gap-2 mb-1">
                <span className="font-semibold text-sm text-gray-800">
                  {step.title || `步骤 ${i + 1}`}
                </span>
                {step.time && (
                  <span className="text-xs text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded">
                    {step.time}
                  </span>
                )}
              </div>
              {step.content && (
                <p
                  className={cn(
                    "text-sm leading-relaxed transition-colors",
                    isActive ? "text-gray-800" : "text-gray-500"
                  )}
                >
                  {step.content}
                </p>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
