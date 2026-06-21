import { motion } from "framer-motion";
import { Sparkles } from "lucide-react";

interface GeneratingViewProps {
  tokens: string;
}

export default function GeneratingView({ tokens }: GeneratingViewProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-3"
    >
      <div className="flex items-center gap-2 text-xhs">
        <motion.div
          animate={{ rotate: [0, 15, -15, 0] }}
          transition={{ repeat: Infinity, duration: 2, ease: "easeInOut" }}
        >
          <Sparkles size={20} />
        </motion.div>
        <span className="font-semibold text-sm">AI 正在提炼菜谱...</span>
      </div>

      <div className="bg-gray-900 rounded-xl p-4 font-mono text-sm text-green-400 h-64 overflow-y-auto whitespace-pre-wrap leading-relaxed shadow-inner">
        {tokens || (
          <motion.span
            animate={{ opacity: [1, 0.3, 1] }}
            transition={{ repeat: Infinity, duration: 1 }}
          >
            ▊
          </motion.span>
        )}
        {tokens && (
          <motion.span
            animate={{ opacity: [1, 0] }}
            transition={{ repeat: Infinity, duration: 0.8, repeatType: "reverse" }}
          >
            ▊
          </motion.span>
        )}
      </div>
    </motion.div>
  );
}
