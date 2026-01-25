import { LucideIcon } from "lucide-react";

interface InfoCardProps {
  icon: LucideIcon;
  title: string;
  description: string;
}

export function InfoCard({ icon: Icon, title, description }: InfoCardProps) {
  return (
    <div className="bg-app-panel border border-app-border rounded-xl p-5 flex gap-4 shadow-md">
      <div className="w-10 h-10 rounded-lg bg-app-card shrink-0 flex items-center justify-center text-app-subtext">
        <Icon className="w-5 h-5" />
      </div>
      <div>
        <h4 className="text-sm font-bold text-app-text mb-1 uppercase tracking-tight">{title}</h4>
        <p className="text-xs text-app-subtext leading-relaxed">{description}</p>
      </div>
    </div>
  );
}
