import type { Step } from "@/types/recipe";
import { STEP_NUMS } from "@/lib/helpers";

interface StepListProps {
  steps: Step[];
}

export default function StepList({ steps }: StepListProps) {
  return (
    <div className="mb-4">
      <div className="font-bold text-gray-600 text-sm mb-2">
        📝 步骤
      </div>
      {steps.map((step, i) => {
        const num = STEP_NUMS[i] || `${i + 1}.`;
        return (
          <div key={i} className="mb-2 pl-0">
            <div className="flex items-baseline gap-1 flex-wrap">
              <span className="font-bold text-[15px] text-xhs">{num}</span>
              <span className="font-semibold text-sm text-gray-800">
                {step.title}
              </span>
              {step.time && (
                <span className="text-[13px] text-gray-400">
                  （{step.time}）
                </span>
              )}
            </div>
            <p className="text-sm text-gray-600 ml-5 mt-0.5 leading-relaxed">
              {step.content}
            </p>
          </div>
        );
      })}
    </div>
  );
}
