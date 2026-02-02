export default function VisualizeJsonGlobalStyles() {
  return (
    <style>{`
      .no-scrollbar::-webkit-scrollbar {
        display: none;
      }
      .no-scrollbar {
        -ms-overflow-style: none;
        scrollbar-width: none;
      }

      .custom-scrollbar::-webkit-scrollbar {
        width: 8px;
        height: 8px;
      }
      .custom-scrollbar::-webkit-scrollbar-track {
        background: transparent;
      }
      .custom-scrollbar::-webkit-scrollbar-thumb {
        background: var(--color-app-border);
        border-radius: 4px;
      }
      .custom-scrollbar::-webkit-scrollbar-thumb:hover {
        background: var(--color-app-subtext);
      }

      .json-formatter {
        line-height: 1.6;
      }

      .json-formatter::-webkit-scrollbar {
        width: 8px;
        height: 8px;
      }

      .json-formatter::-webkit-scrollbar-track {
        background: transparent;
      }

      .json-formatter::-webkit-scrollbar-thumb {
        background: var(--color-app-border);
        border-radius: 4px;
      }

      .json-formatter::-webkit-scrollbar-thumb:hover {
        background: var(--color-app-subtext);
      }
    `}</style>
  );
}
