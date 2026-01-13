import { useState, type FormEvent } from "react";
import { MessageSquare, Send, User, Mail, MessageCircle } from "lucide-react";
import { Button } from "../../shared/components/Button";

export default function FeedbackTab() {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [message, setMessage] = useState("");

  const handleSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    console.log("Feedback submitted:", { name, email, message });
    // Reset form or show success message
    alert("Thank you for your feedback!");
    setName("");
    setEmail("");
    setMessage("");
  };

  return (
    <div className="flex bg-app-bg text-app-text min-h-full p-8 justify-center overflow-y-auto">
      <div className="max-w-xl w-full space-y-8">
        <div className="text-center space-y-2">
          <div className="inline-flex p-3 rounded-2xl bg-app-accent/10 text-app-accent border border-app-accent/20 mb-2">
            <MessageSquare className="w-8 h-8" />
          </div>
          <h2 className="text-2xl font-bold tracking-tight">User Feedback</h2>
          <p className="text-app-subtext text-sm">
            Help us improve gadogado by sharing your thoughts or reporting
            issues.
          </p>
        </div>

        <form
          onSubmit={handleSubmit}
          className="bg-app-card border border-app-border rounded-xl p-6 shadow-sm space-y-5">
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-app-subtext flex items-center gap-2">
                <User className="w-3.5 h-3.5" /> Name
              </label>
              <input
                type="text"
                value={name}
                onInput={(e) => setName((e.target as HTMLInputElement).value)}
                placeholder="Your Name"
                className="w-full bg-app-bg border border-app-border rounded-lg px-4 py-2.5 text-sm focus:border-app-accent outline-none transition"
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-app-subtext flex items-center gap-2">
                <Mail className="w-3.5 h-3.5" /> Email Address
              </label>
              <input
                type="email"
                value={email}
                onInput={(e) => setEmail((e.target as HTMLInputElement).value)}
                placeholder="your@email.com"
                className="w-full bg-app-bg border border-app-border rounded-lg px-4 py-2.5 text-sm focus:border-app-accent outline-none transition"
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-app-subtext flex items-center gap-2">
                <MessageCircle className="w-3.5 h-3.5" /> Message
              </label>
              <textarea
                rows={4}
                value={message}
                onInput={(e) =>
                  setMessage((e.target as HTMLTextAreaElement).value)
                }
                placeholder="Tell us what's on your mind..."
                className="w-full bg-app-bg border border-app-border rounded-lg px-4 py-2.5 text-sm focus:border-app-accent outline-none transition resize-none"></textarea>
            </div>
          </div>

          <Button
            type="submit"
            className="w-full gap-2 h-11 bg-app-accent hover:bg-app-accent/90 text-white rounded-lg font-semibold">
            <Send className="w-4 h-4" /> Send Feedback
          </Button>
        </form>

        <div className="pt-4 border-t border-app-border/50 text-center">
          <p className="text-[10px] text-app-subtext uppercase tracking-widest font-bold">
            Community & Support
          </p>
          <div className="flex justify-center gap-6 mt-4">
            <a href="#" className="text-xs text-app-accent hover:underline">
              GitHub Discussions
            </a>
            <a href="#" className="text-xs text-app-accent hover:underline">
              Discord Server
            </a>
            <a href="#" className="text-xs text-app-accent hover:underline">
              Email Support
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
