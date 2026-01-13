import { useState, useEffect } from "react";
import { Camera, RefreshCcw, ChevronLeft, ChevronRight, Maximize2, Play, Pause, Grid, Square } from "lucide-react";
import ScreenshotLightbox from "./ScreenshotLightbox";
import { cn } from "../../../utils/cn";

type ScreenshotItem = {
  id: string;
  src: string;
  ts: number;
};

type ScreenshotGalleryCardProps = {
  sessionLoading: boolean;
  screenshotLoading: boolean;
  screenshots: ScreenshotItem[];
  screenshotError: string | null;
  onRetryCapture: () => void;
};

export default function ScreenshotGalleryCard({
  sessionLoading,
  screenshotLoading,
  screenshots,
  screenshotError,
  onRetryCapture,
}: ScreenshotGalleryCardProps) {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [isLightboxOpen, setIsLightboxOpen] = useState(false);
  const [viewMode, setViewMode] = useState<"single" | "grid">("single");
  const [isPlaying, setIsPlaying] = useState(false);
  const [playbackSpeed, setPlaybackSpeed] = useState(500); // ms per frame

  // When new screenshots arrive, if we were at the end, jump to the new latest
  useEffect(() => {
    if (screenshots.length > 0 && !isPlaying && viewMode === "single") {
        if (currentIndex >= screenshots.length) {
            setCurrentIndex(screenshots.length - 1);
        }
    }
  }, [screenshots.length, isPlaying, viewMode]);

  // Slideshow logic
  useEffect(() => {
    let interval: ReturnType<typeof setInterval>;
    if (isPlaying && screenshots.length > 1) {
      interval = setInterval(() => {
        setCurrentIndex((prev) => (prev + 1) % screenshots.length);
      }, playbackSpeed);
    }
    return () => clearInterval(interval);
  }, [isPlaying, screenshots.length, playbackSpeed]);

  const goToPrev = () => {
    setCurrentIndex((prev) => (prev > 0 ? prev - 1 : screenshots.length - 1));
  };

  const goToNext = () => {
    setCurrentIndex((prev) => (prev < screenshots.length - 1 ? prev + 1 : 0));
  };

  const togglePlay = () => setIsPlaying(!isPlaying);

  const currentScreenshot = screenshots[currentIndex];

  const handleGridClick = (index: number) => {
    setCurrentIndex(index);
    // setViewMode("single"); // Optional: switch to single view or open lightbox directly?
    setIsLightboxOpen(true);
  };

  return (
    <>
      <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm h-full flex flex-col">
        <div className="flex items-center justify-between gap-2 mb-3">
          <div className="flex items-center gap-2 text-app-text font-medium text-sm">
            <Camera className="w-4 h-4 text-amber-300" />
            <h4>Screenshots {screenshots.length > 0 && <span className="text-app-subtext font-normal ml-1">({screenshots.length})</span>}</h4>
          </div>
          
          <div className="flex items-center gap-1.5">
              {/* Playback Controls */}
              {screenshots.length > 1 && (
                  <div className="flex items-center gap-1 bg-black/20 p-1 rounded-md border border-app-border/50 mr-2">
                       <button
                         onClick={togglePlay}
                         className={cn(
                             "p-1.5 rounded transition hover:bg-white/10",
                             isPlaying ? "text-emerald-400" : "text-app-subtext"
                         )}
                         title={isPlaying ? "Pause Slideshow" : "Play Slideshow"}
                       >
                           {isPlaying ? <Pause className="w-3.5 h-3.5" /> : <Play className="w-3.5 h-3.5" />}
                       </button>
                       <div className="h-4 w-px bg-app-border/50 mx-0.5" />
                       <button
                         onClick={() => setPlaybackSpeed(1000)}
                         className={cn("text-[9px] font-bold px-1.5 py-0.5 rounded", playbackSpeed === 1000 ? "text-emerald-400 bg-emerald-500/10" : "text-app-subtext hover:text-white")}
                         title="Slow (1s)"
                       >1x</button>
                       <button
                         onClick={() => setPlaybackSpeed(500)}
                         className={cn("text-[9px] font-bold px-1.5 py-0.5 rounded", playbackSpeed === 500 ? "text-emerald-400 bg-emerald-500/10" : "text-app-subtext hover:text-white")}
                         title="Normal (0.5s)"
                       >2x</button>
                       <button
                         onClick={() => setPlaybackSpeed(100)}
                         className={cn("text-[9px] font-bold px-1.5 py-0.5 rounded", playbackSpeed === 100 ? "text-emerald-400 bg-emerald-500/10" : "text-app-subtext hover:text-white")}
                         title="Fast (0.1s)"
                       >5x</button>
                  </div>
              )}

              {/* View Mode Toggle */}
              <div className="flex bg-black/20 p-0.5 rounded-md border border-app-border/50">
                  <button
                    onClick={() => setViewMode("single")}
                    className={cn(
                        "p-1.5 rounded transition",
                        viewMode === "single" ? "bg-app-panel text-app-text shadow-sm" : "text-app-subtext hover:text-app-text"
                    )}
                    title="Single View"
                  >
                      <Square className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={() => setViewMode("grid")}
                    className={cn(
                        "p-1.5 rounded transition",
                        viewMode === "grid" ? "bg-app-panel text-app-text shadow-sm" : "text-app-subtext hover:text-app-text"
                    )}
                    title="Grid View"
                  >
                      <Grid className="w-3.5 h-3.5" />
                  </button>
              </div>
          </div>
        </div>

        {viewMode === "single" ? (
             <div className="relative group flex-1 min-h-[300px] flex flex-col">
                <div className="mt-0 rounded-md border border-app-border bg-black/30 flex-1 overflow-hidden flex items-center justify-center relative">
                  {sessionLoading && (
                    <div className="text-[11px] text-app-subtext">Loading session...</div>
                  )}
                  
                  {!sessionLoading && screenshotLoading && (
                    <div className="absolute inset-0 bg-black/20 flex items-center justify-center z-10 pointer-events-none">
                      <div className="bg-app-card/80 px-3 py-1 rounded-full text-[10px] text-app-text border border-app-border shadow-lg">
                        Capturing...
                      </div>
                    </div>
                  )}

                  {!sessionLoading && screenshots.length > 0 && currentScreenshot && (
                    <>
                      <img
                        src={currentScreenshot.src}
                        alt={`Screenshot ${currentIndex + 1}`}
                        className="w-full h-full object-contain cursor-zoom-in"
                        onClick={() => setIsLightboxOpen(true)}
                      />
                      
                      {/* Overlay info */}
                      <div className="absolute bottom-2 left-2 right-2 flex justify-between items-end opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
                        <div className="bg-black/60 backdrop-blur-md px-2 py-1 rounded text-[9px] text-white/80 border border-white/10">
                          {currentIndex + 1}/{screenshots.length} • {new Date(currentScreenshot.ts).toLocaleTimeString()}
                        </div>
                          <div className="flex items-center gap-1.5 pointer-events-auto">
                          <button 
                            className="bg-sky-500 hover:bg-sky-400 backdrop-blur-md p-1.5 rounded border border-white/10 text-white shadow-lg transition-transform hover:scale-105 active:scale-95"
                            onClick={() => setIsLightboxOpen(true)}
                            title="View Full Screen"
                          >
                            <Maximize2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </>
                  )}

                  {!sessionLoading && screenshots.length === 0 && !screenshotLoading && (
                    <div className="text-[11px] text-app-subtext text-center px-6">
                      No screenshots captured yet.
                    </div>
                  )}
                </div>
                
                {screenshots.length > 1 && (
                     <div className="flex items-center justify-between mt-2 px-1">
                        <button onClick={goToPrev} className="p-1 hover:bg-app-border/50 rounded transition text-app-subtext"><ChevronLeft className="w-4 h-4" /></button>
                        <input 
                              type="range" 
                              min={0} 
                              max={screenshots.length - 1} 
                              value={currentIndex} 
                              onChange={(e) => setCurrentIndex(Number(e.target.value))}
                              className="mx-3 flex-1 h-1 bg-app-border rounded-lg appearance-none cursor-pointer accent-emerald-500"
                        />
                        <button onClick={goToNext} className="p-1 hover:bg-app-border/50 rounded transition text-app-subtext"><ChevronRight className="w-4 h-4" /></button>
                     </div>
                )}
             </div>
        ) : (
             /* GRID MODE */
             <div className="flex-1 overflow-y-auto min-h-[300px] border border-app-border/50 rounded-md bg-black/10 p-2 custom-scrollbar">
                 {screenshots.length === 0 ? (
                      <div className="flex h-full items-center justify-center text-[11px] text-app-subtext">No screenshots</div>
                 ) : (
                     <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 gap-2">
                         {screenshots.map((s, idx) => (
                             <button
                               key={s.id}
                               onClick={() => handleGridClick(idx)}
                               className={cn(
                                   "relative aspect-video rounded overflow-hidden border transition-all hover:scale-105 focus:ring-2 focus:ring-emerald-500/50 outline-none",
                                   idx === currentIndex ? "border-emerald-500 ring-1 ring-emerald-500/30" : "border-app-border hover:border-app-subtext"
                               )}
                             >
                                 <img src={s.src} alt="" className="w-full h-full object-cover" loading="lazy" />
                                 <div className="absolute bottom-0 right-0 bg-black/60 text-[8px] text-white/90 px-1 rounded-tl">
                                     #{idx+1}
                                 </div>
                             </button>
                         ))}
                     </div>
                 )}
             </div>
        )}

        {screenshotError && (
          <div className="mt-2 flex items-center gap-2 text-[11px] text-red-200">
            <span className="truncate">{screenshotError}</span>
            <button
              type="button"
              onClick={onRetryCapture}
              className="flex-shrink-0 flex items-center gap-1.5 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
              <RefreshCcw className="w-3 h-3" />
              Retry
            </button>
          </div>
        )}
      </div>

      <ScreenshotLightbox 
        isOpen={isLightboxOpen}
        onClose={() => setIsLightboxOpen(false)}
        screenshots={screenshots}
        currentIndex={currentIndex}
        onNavigate={setCurrentIndex}
      />
    </>
  );
}
