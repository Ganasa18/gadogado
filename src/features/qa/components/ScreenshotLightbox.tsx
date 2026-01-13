import { X, ChevronLeft, ChevronRight, Play, Pause, Gauge } from "lucide-react";
import { useEffect, useState } from "react";
import { cn } from "../../../utils/cn";

type ScreenshotItem = {
  id: string;
  src: string;
  ts: number;
};

type ScreenshotLightboxProps = {
  isOpen: boolean;
  onClose: () => void;
  screenshots: ScreenshotItem[];
  currentIndex: number;
  onNavigate: (index: number) => void;
};

export default function ScreenshotLightbox({
  isOpen,
  onClose,
  screenshots,
  currentIndex,
  onNavigate,
}: ScreenshotLightboxProps) {
  const [isPlaying, setIsPlaying] = useState(false);
  const [playbackSpeed, setPlaybackSpeed] = useState(500);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return;
      if (e.key === "Escape") onClose();
      if (e.key === " " || e.key === "Space") {
          e.preventDefault();
          setIsPlaying(!isPlaying);
      }
      if (e.key === "ArrowLeft") {
          setIsPlaying(false);
          onNavigate((currentIndex - 1 + screenshots.length) % screenshots.length); // Loop around? Or logic in parent?
      }
      if (e.key === "ArrowRight") {
          setIsPlaying(false);
           onNavigate((currentIndex + 1) % screenshots.length);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, currentIndex, screenshots.length, onClose, onNavigate, isPlaying]);

  useEffect(() => {
    let interval: ReturnType<typeof setInterval>;
    if (isPlaying && isOpen && screenshots.length > 1) {
       interval = setInterval(() => {
           onNavigate((currentIndex + 1) % screenshots.length);
       }, playbackSpeed);
    }
    return () => clearInterval(interval);
  }, [isPlaying, isOpen, currentIndex, screenshots.length, onNavigate, playbackSpeed]);

  if (!isOpen || screenshots.length === 0) return null;

  const current = screenshots[currentIndex];

  return (
    <div className="fixed inset-0 z-[100] flex flex-col bg-black/95 backdrop-blur-sm animate-in fade-in duration-200">
      {/* Header */}
      <div className="flex items-center justify-between p-4 text-white hover:opacity-100 opacity-60 transition-opacity">
        <div className="flex flex-col">
          <span className="text-sm font-medium">Screenshot {currentIndex + 1} of {screenshots.length}</span>
          <span className="text-[10px] text-white/60">{new Date(current.ts).toLocaleString()}</span>
        </div>
        <div className="flex items-center gap-3">
          <button 
            onClick={onClose}
            className="p-2 hover:bg-white/10 rounded-full transition"
            title="Close (Esc)"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="relative flex-1 flex items-center justify-center p-4">
        {/* Navigation - Left */}
        <button 
            onClick={(e) => {
               e.stopPropagation();
               setIsPlaying(false);
               onNavigate((currentIndex - 1 + screenshots.length) % screenshots.length);
            }}
            className="absolute left-6 z-10 p-3 bg-white/5 hover:bg-white/15 rounded-full text-white transition-all hover:scale-110 opacity-0 group-hover:opacity-100 md:opacity-100"
        >
            <ChevronLeft className="w-8 h-8 md:w-10 md:h-10" />
        </button>

        <div className="w-full h-full flex items-center justify-center cursor-pointer" onClick={onClose}>
            <img 
              src={current.src} 
              alt={`Full size screenshot ${currentIndex + 1}`}
              className="max-w-full max-h-full object-contain shadow-2xl animate-in zoom-in-95 duration-300 pointer-events-auto select-none"
              onClick={(e) => {
                  e.stopPropagation();
                  setIsPlaying(!isPlaying);
              }}
            />
        </div>

        {/* Navigation - Right */}
        <button 
            onClick={(e) => {
               e.stopPropagation();
               setIsPlaying(false);
               onNavigate((currentIndex + 1) % screenshots.length);
            }}
            className="absolute right-6 z-10 p-3 bg-white/5 hover:bg-white/15 rounded-full text-white transition-all hover:scale-110 opacity-0 group-hover:opacity-100 md:opacity-100"
        >
            <ChevronRight className="w-8 h-8 md:w-10 md:h-10" />
        </button>
      </div>
      
      {/* Footer Controls */}
      <div className="pb-8 flex flex-col items-center gap-4">
         {/* Playback Controls */}
         <div className="flex items-center gap-2 bg-white/10 backdrop-blur-md px-3 py-2 rounded-full border border-white/10">
              <button
                onClick={() => setIsPlaying(!isPlaying)}
                className={cn(
                    "p-2 rounded-full transition hover:bg-white/10",
                    isPlaying ? "text-emerald-400 bg-emerald-500/10" : "text-white"
                )}
                title={isPlaying ? "Pause (Space)" : "Play (Space)"}
              >
                  {isPlaying ? <Pause className="fill-current w-5 h-5" /> : <Play className="fill-current w-5 h-5 ml-0.5" />}
              </button>
              
              <div className="w-px h-6 bg-white/10 mx-1" />
              
              <div className="flex items-center gap-1">
                 <Gauge className="w-3.5 h-3.5 text-white/50" />
                 <button onClick={() => setPlaybackSpeed(1000)} className={cn("text-[10px] font-bold px-2 py-1 rounded transition", playbackSpeed===1000 ? "bg-white/20 text-white" : "text-white/50 hover:text-white")}>1x</button>
                 <button onClick={() => setPlaybackSpeed(500)} className={cn("text-[10px] font-bold px-2 py-1 rounded transition", playbackSpeed===500 ? "bg-white/20 text-white" : "text-white/50 hover:text-white")}>2x</button>
                 <button onClick={() => setPlaybackSpeed(100)} className={cn("text-[10px] font-bold px-2 py-1 rounded transition", playbackSpeed===100 ? "bg-white/20 text-white" : "text-white/50 hover:text-white")}>5x</button>
              </div>
         </div>
         
         <div className="flex items-center gap-3 w-full max-w-md px-8">
             <span className="text-[10px] text-white/60 w-8 text-right">{currentIndex + 1}</span>
             <input 
                type="range" 
                min={0} 
                max={screenshots.length - 1} 
                value={currentIndex} 
                onChange={(e) => {
                    setIsPlaying(false);
                    onNavigate(Number(e.target.value));
                }}
                className="flex-1 h-1 bg-white/20 rounded-lg appearance-none cursor-pointer accent-emerald-500 hover:bg-white/30 transition-colors"
                onClick={(e) => e.stopPropagation()}
            />
             <span className="text-[10px] text-white/60 w-8">{screenshots.length}</span>
         </div>
      </div>
    </div>
  );
}
