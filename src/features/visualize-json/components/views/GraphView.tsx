import { useMemo } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  Panel,
  ReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Layout } from "lucide-react";

import { Button } from "../../../../shared/components/Button";
import CustomGraphNode from "../CustomGraphNode";
import { useGraphLayout } from "../../hooks/useGraphLayout";
import type { JsonNode } from "../../types";

interface GraphViewProps {
  json: JsonNode;
  mode: "dark" | "light" | "system";
  onSwitchToTable: () => void;
}

export default function GraphView({ json, mode, onSwitchToTable }: GraphViewProps) {
  const { nodes: layoutedNodes, edges: layoutedEdges, isTooLarge } = useGraphLayout(json);
  const nodeTypes = useMemo(() => ({ custom: CustomGraphNode }), []);

  const controlsStyle = useMemo(
    () => ({
      controlsBg: mode === "dark" ? "rgba(30, 30, 30, 0.95)" : "rgba(255, 255, 255, 0.95)",
      buttonBg: mode === "dark" ? "#1e1e1e" : "#ffffff",
      textColor: mode === "dark" ? "#ffffff" : "#000000",
    }),
    [mode],
  );

  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={layoutedNodes}
        edges={layoutedEdges}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={4}
        defaultEdgeOptions={{
          type: "smoothstep",
          animated: true,
          style: {
            stroke: mode === "dark" ? "#6b7280" : "#d1d5db",
            strokeWidth: 1.5,
            opacity: 0.6,
          },
        }}
        className="bg-app-bg"
      >
        <Background
          color={mode === "dark" ? "#6b7280" : "#d1d5db"}
          variant={BackgroundVariant.Dots}
          gap={20}
          size={1}
          className="opacity-20"
        />
        <Controls />
        <Panel
          position="top-left"
          className="bg-app-panel/90 border border-app-border p-3 rounded-xl text-[11px] text-app-subtext shadow-lg flex flex-col gap-2"
        >
          <div className="font-bold text-app-text uppercase tracking-widest text-[10px] mb-1">Key Types</div>
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-blue-400"></div>
            <span className="flex-1">Object</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-green-400"></div>
            <span className="flex-1">Array</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-orange-400"></div>
            <span className="flex-1">Value</span>
          </div>
        </Panel>

        {isTooLarge && (
          <div className="absolute inset-0 flex items-center justify-center bg-app-bg/80 z-50 backdrop-blur-sm p-6 text-center">
            <div className="max-w-md bg-app-card border border-app-border p-6 rounded-2xl shadow-2xl">
              <Layout className="w-12 h-12 text-orange-400 mx-auto mb-4" />
              <h3 className="text-lg font-bold text-app-text mb-2">Graph Too Large</h3>
              <p className="text-sm text-app-subtext mb-6">
                Data ini memiliki lebih dari 150 node yang terlihat. Merendernya sebagai graph akan membuat aplikasi sangat lambat.
                Silakan gunakan <b>Table</b> atau <b>Tree view</b> untuk performa lebih baik, atau tutup beberapa node folder.
              </p>
              <div className="flex gap-3 justify-center">
                <Button onClick={onSwitchToTable} className="bg-app-accent text-white">
                  Switch to Table View
                </Button>
              </div>
            </div>
          </div>
        )}
      </ReactFlow>

      <style>{`
        .react-flow__controls {
          background: ${controlsStyle.controlsBg} !important;
          border: 1px solid ${mode === "dark" ? "#374151" : "#e5e7eb"} !important;
          backdrop-filter: blur(8px) !important;
          border-radius: 8px !important;
        }

        .react-flow__controls-button {
          background: ${controlsStyle.buttonBg} !important;
          border-bottom: 1px solid ${mode === "dark" ? "#374151" : "#e5e7eb"} !important;
          color: ${controlsStyle.textColor} !important;
          fill: ${controlsStyle.textColor} !important;
        }

        .react-flow__controls-button:hover {
          background: var(--color-app-accent, #3b82f6) !important;
          color: white !important;
          fill: white !important;
        }

        .react-flow__controls-button:last-child {
          border-bottom: none !important;
        }

        .react-flow__controls-button svg {
          width: 16px !important;
          height: 16px !important;
        }
      `}</style>
    </div>
  );
}
