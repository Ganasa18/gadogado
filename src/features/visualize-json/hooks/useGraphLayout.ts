 
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
  const [isTooLarge, setIsTooLarge] = useState(false);

  useEffect(() => {
    if (!json) {
      setLayoutedNodes([]);
      setLayoutedEdges([]);
      return;
    }

    // Performance safety check: Skip layouting if there are too many visible nodes
    let nodeCount = 0;
    const countNodes = (node: JsonNode) => {
      nodeCount++;
      if (node.expanded !== false && node.children) {
        node.children.forEach(countNodes);
      }
    };
    countNodes(json);

    if (nodeCount > 150) {
      setIsTooLarge(true);
      return;
    }
    setIsTooLarge(false);

    const nodes: any[] = [];
    const edges: any[] = [];
    const elkNodes: any[] = [];
    const elkEdges: any[] = [];

    const walk = (node: JsonNode, parentExpanded: boolean = true) => {
      if (!parentExpanded && node.depth !== 0) return;

      nodes.push({
        id: node.path,
        type: 'custom',
        data: {
          label: node.key || 'root',
          type: node.type,
          value: getValuePreview(node),
          rawValue: node.value,
          isRoot: node.depth === 0,
          expanded: node.expanded !== false,
          hasChildren: !!node.children && node.children.length > 0,
          path: node.path,
        },
        position: { x: 0, y: 0 },
      });

      elkNodes.push({
        id: node.path,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
      });

      if (node.children && node.expanded !== false) {
        node.children.forEach((child) => {
          edges.push({
            id: `${node.path}-${child.path}`,
            source: node.path,
            target: child.path,
            type: 'smoothstep',
            animated: true,
            markerEnd: {
              type: MarkerType.ArrowClosed,
              color: 'var(--color-app-accent)',
            },
            style: { stroke: 'var(--color-app-border)', strokeWidth: 1.5, opacity: 0.6 }
          });

          elkEdges.push({
            id: `${node.path}-${child.path}`,
            sources: [node.path],
            targets: [child.path],
          });

          walk(child, true);
        });
      }
    };

    walk(json, true);

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

  return { nodes: layoutedNodes, edges: layoutedEdges, isTooLarge };
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
