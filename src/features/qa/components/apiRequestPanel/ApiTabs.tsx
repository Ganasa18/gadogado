type ApiTabsProps = {
  activeTab: "params" | "headers" | "body" | "response";
  onChange: (tab: "params" | "headers" | "body" | "response") => void;
};

export function ApiTabs({ activeTab, onChange }: ApiTabsProps) {
  const tabs = ["params", "headers", "body", "response"] as const;
  return (
    <div className="flex flex-wrap items-center gap-2 border-b border-app-border pb-2 text-[10px]">
      {tabs.map((tab) => (
        <button
          key={tab}
          type="button"
          onClick={() => onChange(tab)}
          className={`px-3 py-1 rounded border transition ${
            activeTab === tab
              ? "bg-emerald-700/20 border-emerald-500/60 text-emerald-100"
              : "bg-[#181818] border-app-border text-app-subtext"
          }`}>
          {tab.toUpperCase()}
        </button>
      ))}
    </div>
  );
}
