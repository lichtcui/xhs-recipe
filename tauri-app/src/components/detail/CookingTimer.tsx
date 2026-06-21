import { useState, useEffect, useCallback, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Timer, X } from "lucide-react";

const PRESETS = [1, 3, 5, 10, 15, 30];

export default function CookingTimer() {
  const [open, setOpen] = useState(false);
  const [seconds, setSeconds] = useState(0);
  const [running, setRunning] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const start = useCallback((minutes: number) => {
    setSeconds(minutes * 60);
    setRunning(true);
  }, []);

  useEffect(() => {
    if (running && seconds > 0) {
      intervalRef.current = setInterval(() => {
        setSeconds((s) => {
          if (s <= 1) {
            setRunning(false);
            return 0;
          }
          return s - 1;
        });
      }, 1000);
    }
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [running, seconds > 0]);

  // Timer finished notification
  useEffect(() => {
    if (!running && seconds === 0 && open) {
      // Could use Tauri notification here
    }
  }, [running, seconds, open]);

  const stop = () => {
    if (intervalRef.current) clearInterval(intervalRef.current);
    setRunning(false);
    setSeconds(0);
  };

  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;

  if (!open) {
    return (
      <Button
        variant="outline"
        size="sm"
        onClick={() => setOpen(true)}
        className="text-xs"
      >
        <Timer size={14} className="mr-1" />
        计时器
      </Button>
    );
  }

  return (
    <div className="fixed bottom-20 left-1/2 -translate-x-1/2 z-50 bg-white rounded-2xl shadow-2xl border p-4 w-72">
      <div className="flex items-center justify-between mb-3">
        <span className="text-sm font-semibold">烹饪计时</span>
        <button onClick={() => { stop(); setOpen(false); }} className="text-gray-400 hover:text-gray-600">
          <X size={16} />
        </button>
      </div>

      {running || seconds > 0 ? (
        <div className="text-center mb-3">
          <div className="text-4xl font-mono font-bold text-xhs tabular-nums">
            {String(mins).padStart(2, "0")}:{String(secs).padStart(2, "0")}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={stop}
            className="mt-2 text-xs"
          >
            停止
          </Button>
        </div>
      ) : (
        <div className="grid grid-cols-3 gap-1.5">
          {PRESETS.map((m) => (
            <Button
              key={m}
              variant="outline"
              size="sm"
              onClick={() => start(m)}
              className="text-xs h-8"
            >
              {m}分钟
            </Button>
          ))}
        </div>
      )}
    </div>
  );
}
