import { useState, useEffect } from "react";
import { MarkdownRenderer } from "./MarkdownRenderer";

interface TypewriterTextProps {
  content: string;
  speed?: number;
  onComplete?: () => void;
}

export function TypewriterText({ content, speed = 4, onComplete }: TypewriterTextProps) {
  const [displayedContent, setDisplayedContent] = useState("");
  const [index, setIndex] = useState(0);

  useEffect(() => {
    if (index < content.length) {
      const timeout = setTimeout(() => {
        // Type multiple characters at once for faster feeling on long text
        const increment = index + 5 > content.length ? content.length - index : 5;
        setDisplayedContent((prev) => prev + content.slice(index, index + increment));
        setIndex((prev) => prev + increment);
      }, speed);
      return () => clearTimeout(timeout);
    } else if (onComplete) {
      onComplete();
    }
  }, [index, content, speed, onComplete]);

  return (
    <div className="typewriter-container">
      <MarkdownRenderer content={displayedContent} />
      {index < content.length && <span className="inline-block w-1.5 h-4 ml-1 bg-app-accent animate-pulse" />}
    </div>
  );
}
