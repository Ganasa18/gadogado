import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import remarkBreaks from "remark-breaks";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneDark } from "react-syntax-highlighter/dist/esm/styles/prism";
import { Copy, Check } from "lucide-react";
import { useState, useCallback } from "react";

interface Props {
  content: string;
}

export function MarkdownRenderer({ content }: Props) {
  const [copiedCode, setCopiedCode] = useState<string | null>(null);

  const handleCopyCode = useCallback((code: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCode(code);
    setTimeout(() => setCopiedCode(null), 2000);
  }, []);

  return (
    <ReactMarkdown
      remarkPlugins={[
        remarkGfm,      // GitHub Flavored Markdown (tables, strikethrough, autolinks, etc.)
        remarkMath,     // Math equations support
        remarkBreaks,   // Convert single newlines to <br> for better formatting
      ]}
      components={{
        // Code blocks with syntax highlighting
        code(props: any) {
          const { className, children } = props;
          const match = /language-(\w+)/.exec(className || "");
          const codeString = String(children).replace(/\n$/, "");
          const inline = (props as any).inline;

          if (!inline && match) {
            return (
              <div className="relative group my-3">
                <div className="absolute right-2 top-2 z-10 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button
                    onClick={() => handleCopyCode(codeString)}
                    className="p-1.5 rounded bg-app-bg/80 text-app-text-muted hover:text-app-text transition-colors"
                    title="Copy code">
                    {copiedCode === codeString ? (
                      <Check className="w-3.5 h-3.5 text-green-500" />
                    ) : (
                      <Copy className="w-3.5 h-3.5" />
                    )}
                  </button>
                </div>
                <SyntaxHighlighter
                  style={oneDark}
                  language={match[1]}
                  PreTag="div"
                  customStyle={{
                    margin: 0,
                    borderRadius: "0.5rem",
                    fontSize: "0.8rem",
                    maxHeight: "400px",
                  } as any}
                  {...props}>
                  {codeString}
                </SyntaxHighlighter>
              </div>
            );
          }

          // Inline code
          return (
            <code
              className="px-1.5 py-0.5 rounded bg-app-bg/50 text-app-accent text-[0.85em] font-mono"
              {...props}>
              {children}
            </code>
          );
        },

        // Headers
        h1: ({ children }: any) => (
          <h1 className="text-xl font-bold text-app-text mt-4 mb-2">{children}</h1>
        ),
        h2: ({ children }: any) => (
          <h2 className="text-lg font-semibold text-app-text mt-3 mb-2">{children}</h2>
        ),
        h3: ({ children }: any) => (
          <h3 className="text-base font-semibold text-app-text mt-2 mb-1">{children}</h3>
        ),

        // Paragraphs
        p: ({ children }: any) => (
          <p className="text-sm leading-7 text-app-text mb-2 last:mb-0 select-text cursor-text">
            {children}
          </p>
        ),

        // Lists
        ul: ({ children }: any) => (
          <ul className="list-disc list-inside text-sm space-y-1 mb-2 ml-2 select-text">
            {children}
          </ul>
        ),
        ol: ({ children }: any) => (
          <ol className="list-decimal list-inside text-sm space-y-1 mb-2 ml-2 select-text">
            {children}
          </ol>
        ),
        li: ({ children }: any) => (
          <li className="text-app-text leading-relaxed">{children}</li>
        ),

        // Links
        a: ({ href, children }: any) => (
          <a
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            className="text-app-accent hover:underline">
            {children}
          </a>
        ),

        // Blockquotes
        blockquote: ({ children }: any) => (
          <blockquote className="border-l-2 border-app-accent/50 pl-4 italic text-app-text-muted my-2 select-text">
            {children}
          </blockquote>
        ),

        // Horizontal rule
        hr: () => <hr className="border-app-border/30 my-4" />,

        // Tables - Clean & Professional Style
        table: ({ children }: any) => (
          <div className="overflow-x-auto my-4 rounded-lg border border-app-border/30">
            <table className="min-w-full border-collapse text-sm">
              {children}
            </table>
          </div>
        ),
        thead: ({ children }: any) => (
          <thead className="bg-app-card/80 border-b border-app-border/40">
            {children}
          </thead>
        ),
        tbody: ({ children }: any) => (
          <tbody className="divide-y divide-app-border/20 bg-app-bg/50">
            {children}
          </tbody>
        ),
        tr: ({ children }: any) => (
          <tr className="hover:bg-app-card/40 transition-colors">
            {children}
          </tr>
        ),
        th: ({ children }: any) => (
          <th className="px-4 py-2.5 text-left text-[11px] font-bold uppercase tracking-wider text-app-subtext whitespace-nowrap border-r border-app-border/10 last:border-r-0">
            {children}
          </th>
        ),
        td: ({ children }: any) => (
          <td className="px-4 py-2.5 text-[12px] text-app-text whitespace-nowrap border-r border-app-border/10 last:border-r-0">
            {children}
          </td>
        ),

        // Strong/Bold
        strong: ({ children }: any) => (
          <strong className="font-semibold text-app-text">{children}</strong>
        ),

        // Emphasis/Italic
        em: ({ children }: any) => <em className="italic">{children}</em>,

        // Strikethrough (GFM)
        del: ({ children }: any) => (
          <del className="line-through text-app-subtext">{children}</del>
        ),

        // Images
        img: ({ src, alt }: any) => (
          <img
            src={src}
            alt={alt}
            className="max-w-full h-auto rounded-lg my-2 border border-app-border/20"
          />
        ),

        // Pre (for code blocks without language)
        pre: ({ children }: any) => (
          <pre className="bg-app-card rounded-lg p-4 overflow-x-auto my-3 text-sm border border-app-border/30">
            {children}
          </pre>
        ),
      }}>
      {content}
    </ReactMarkdown>
  );
}
