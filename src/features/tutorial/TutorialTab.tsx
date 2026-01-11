import {
  BookOpen,
  PlayCircle,
  Lightbulb,
  MessageSquare,
  ShieldCheck,
  Sparkles,
  Keyboard,
} from "lucide-react";
import { Button } from "../../shared/components/Button";

export default function TutorialTab() {
  const steps = [
    {
      title: "Setup your LLM",
      desc: "Go to Settings and choose Local (Free) or Cloud (API). For Local, ensure LM Studio is running.",
      icon: <PlayCircle className="w-5 h-5" />,
    },
    {
      title: "Translate Prompts",
      desc: "Type your Indonesian ideas. We use advanced prompt engineering to ensure the English result is optimized for LLMs.",
      icon: <MessageSquare className="w-5 h-5" />,
    },
    {
      title: "Enhance Results",
      desc: "Use the Enhance tool to add structure, roles, and constraints to your English prompts automatically.",
      icon: <Sparkles className="w-5 h-5" />,
    },
  ];

  return (
    <div className="max-w-4xl p-6 mx-auto space-y-12 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="text-center space-y-4">
        <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-primary/10 text-primary text-xs font-bold uppercase tracking-widest border border-primary/20">
          <Lightbulb className="w-3 h-3" /> Quick Start Guide
        </div>
        <h3 className="text-4xl font-extrabold tracking-tight">
          Master PromptBridge
        </h3>
        <p className="text-muted-foreground max-w-xl mx-auto">
          Unlock the full potential of AI with high-quality cross-language
          prompt engineering.
        </p>
      </div>

      <div className="grid md:grid-cols-3 gap-6">
        {steps.map((s, i) => (
          <div
            key={i}
            className="relative p-8 bg-card border border-border rounded-[2rem] space-y-4 shadow-sm hover:shadow-xl hover:-translate-y-1 transition-all duration-300 group">
            <div className="absolute -top-4 -left-4 w-10 h-10 bg-primary text-primary-foreground rounded-full flex items-center justify-center font-bold shadow-lg border-4 border-background">
              {i + 1}
            </div>
            <div className="w-12 h-12 rounded-2xl bg-primary/5 text-primary flex items-center justify-center group-hover:scale-110 transition-transform duration-500">
              {s.icon}
            </div>
            <h4 className="font-bold text-xl">{s.title}</h4>
            <p className="text-sm text-muted-foreground leading-relaxed">
              {s.desc}
            </p>
          </div>
        ))}
      </div>

      <div className="bg-card border border-border rounded-[2rem] p-8 shadow-sm space-y-4">
        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-widest text-primary">
          <Keyboard className="w-4 h-4" /> Shortcut Actions
        </div>
        <p className="text-sm text-muted-foreground">
          Your keyboard actions update dynamically when you record new bindings.
        </p>
      </div>

      <div className="bg-gradient-to-br from-primary to-primary-foreground p-1 rounded-[2.5rem] shadow-2xl shadow-primary/20">
        <div className="bg-background/95 backdrop-blur-sm p-10 rounded-[2.3rem] flex flex-col md:flex-row items-center gap-10">
          <div className="space-y-6 flex-1 text-center md:text-left">
            <h4 className="text-2xl font-bold tracking-tight">
              Privacy First Architecture
            </h4>
            <p className="text-sm text-muted-foreground leading-relaxed">
              PromptBridge never sends your API keys or history to any cloud
              server. Everything stays on your local machine, encrypted and
              secure.
            </p>
            <div className="flex items-center gap-4 justify-center md:justify-start">
              <div className="flex items-center gap-2 text-xs font-bold text-green-600 dark:text-green-400">
                <ShieldCheck className="w-4 h-4" /> Native OS Keychain
              </div>
              <div className="flex items-center gap-2 text-xs font-bold text-green-600 dark:text-green-400">
                <BookOpen className="w-4 h-4" /> Open Source Principles
              </div>
            </div>
          </div>
          <Button
            size="lg"
            className="h-16 px-10 rounded-2xl text-lg font-bold group">
            Start Engineering
            <Sparkles className="ml-2 w-5 h-5 group-hover:rotate-12 transition-transform" />
          </Button>
        </div>
      </div>
    </div>
  );
}
