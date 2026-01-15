
import ELK from 'elkjs/lib/elk.bundled.js';
import { useState, useEffect } from 'react';
import { JsonNode } from '../types';
import { MarkerType } from '@xyflow/react';

const elk = new ELK();

const NODE_WIDTH = 260;
const NODE_HEIGHT = 100;

const elkOptions = {
  'elk.algorithm': 'layered',
  'elk.direction': 'RIGHT',
  'elk.layered.spacing.nodeNodeLayered': '100',
  'elk.layered.nodePlacement.strategy': 'SIMPLE',
  'elk.spacing.nodeNode': '40',
};

export const useGraphLayout = (json: JsonNode | null) => {
  const [layoutedNodes, setLayoutedNodes] = useState<any[]>([]);
  const [layoutedEdges, setLayoutedEdges] = useState<any[]>([]);

  useEffect(() => {
    if (!json) {
      setLayoutedNodes([]);
      setLayoutedEdges([]);
      return;
    }

    const nodes: any[] = [];
    const edges: any[] = [];
    const elkNodes: any[] = [];
    const elkEdges: any[] = [];

    const walk = (node: JsonNode) => {
      nodes.push({
        id: node.path,
        type: 'custom',
        data: {
          label: node.key || 'root',
          type: node.type,
          value: getValuePreview(node),
          isRoot: node.depth === 0,
        },
        position: { x: 0, y: 0 },
      });

      elkNodes.push({
        id: node.path,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
      });

      if (node.children) {
        node.children.forEach((child) => {
          edges.push({
            id: `${node.path}-${child.path}`,
            source: node.path,
            target: child.path,
            type: 'smoothstep',
            animated: true,
            markerEnd: {
              type: MarkerType.ArrowClosed,
              color: '#3b82f6',
            },
            style: { stroke: '#3b82f6', strokeWidth: 1.5, opacity: 0.6 }
          });

          elkEdges.push({
            id: `${node.path}-${child.path}`,
            sources: [node.path],
            targets: [child.path],
          });

          walk(child);
        });
      }
    };

    walk(json);

    const graph = {
      id: 'root',
      layoutOptions: elkOptions,
      children: elkNodes,
      edges: elkEdges,
    };

    elk.layout(graph)
      .then((layoutedGraph) => {
        const finalNodes = nodes.map((node) => {
          const elkNode = layoutedGraph.children?.find((n) => n.id === node.id);
          if (elkNode) {
            return {
              ...node,
              position: { x: elkNode.x || 0, y: elkNode.y || 0 },
            };
          }
          return node;
        });

        setLayoutedNodes(finalNodes);
        setLayoutedEdges(edges);
      })
      .catch(console.error);
  }, [json]);

  return { nodes: layoutedNodes, edges: layoutedEdges };
};

function getValuePreview(node: JsonNode) {
  if (node.type === "object") {
    return `{${node.children?.length ?? 0} keys}`;
  }
  if (node.type === "array") {
    return `[${node.children?.length ?? 0} items]`;
  }
  if (node.type === "string") {
    return `"${node.value}"`;
  }
  if (node.type === "boolean") {
    return String(node.value);
  }
  if (node.type === "null") {
    return "null";
  }
  return String(node.value ?? '');
}
