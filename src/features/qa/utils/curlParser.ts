export type ParsedCurl = {
  method: string;
  url: string;
  headers: { key: string; value: string }[];
  body?: string;
};

export function parseCurlCommand(command: string): ParsedCurl | null {
  if (!command || !command.trim().toLowerCase().startsWith("curl")) return null;

  const args: string[] = [];
  // Regex to match arguments handling quotes
  // Matches: single-quoted string, double-quoted string, or non-whitespace sequence
  const result = command.matchAll(/(".*?"|'.*?'|[^\s"']+)+/g);
  for (const match of result) {
    let arg = match[0];
    // Strip quotes
    if ((arg.startsWith('"') && arg.endsWith('"')) || (arg.startsWith("'") && arg.endsWith("'"))) {
      arg = arg.slice(1, -1);
    }
    args.push(arg);
  }

  let method = "GET";
  let url = "";
  const headers: { key: string; value: string }[] = [];
  let body: string | undefined = undefined;

  for (let i = 1; i < args.length; i++) {
    const arg = args[i];

    if (arg === "-X" || arg === "--request") {
      if (i + 1 < args.length) {
        method = args[++i].toUpperCase();
      }
      continue;
    }

    if (arg === "-H" || arg === "--header") {
      if (i + 1 < args.length) {
        const headerStr = args[++i];
        const colonIndex = headerStr.indexOf(":");
        if (colonIndex !== -1) {
          const key = headerStr.slice(0, colonIndex).trim();
          const value = headerStr.slice(colonIndex + 1).trim();
          headers.push({ key, value });
        }
      }
      continue;
    }

    if (arg === "-d" || arg === "--data" || arg === "--data-raw" || arg === "--data-binary") {
      if (i + 1 < args.length) {
        body = args[++i];
        if (method === "GET") method = "POST";
      }
      continue;
    }
    
    // Ignore other flags for now (-s, -v, etc.)
    if (arg.startsWith("-")) {
        // skip next arg if it's a flag that takes a value?
        // simple heuristic: if known flag, skip.
        // For now, assume unknown flags might take an arg if we see it?
        // Better: Only capture URL if it looks like one and we haven't found one yet.
        continue;
    }

    if (!url && !arg.startsWith("-")) {
      url = arg;
    }
  }

  return { method, url, headers, body };
}
