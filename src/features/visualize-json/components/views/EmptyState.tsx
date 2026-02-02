import { Share2 } from "lucide-react";

export default function EmptyState() {
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center text-app-subtext bg-app-bg/50 z-10">
      <div className="w-20 h-20 bg-app-card rounded-3xl flex items-center justify-center mb-6 border border-app-border shadow-lg">
        <Share2 className="text-app-accent opacity-50" size={40} />
      </div>
      <p className="text-xl font-bold tracking-tight text-app-text">Ready to Visualize</p>
      <p className="text-sm text-app-subtext mt-2">Paste your JSON data on the left to start exploration.</p>
    </div>
  );
}
