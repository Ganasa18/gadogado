# VISUALIZE JSON INTERACTIVE GRAPH

## Goal

Create an interactive graph viewer that turns JSON into a navigable node-link diagram so users can understand structure, nesting, and relationships quickly.

## Core Features

- Load JSON from file or paste (URL optional).
- Convert to JSON into a graph model (nodes + edges).
- Render on a canvas with pan, zoom, and drag.
- Expand/collapse nodes for objects and arrays.
- Search by key, value, or JSONPath and highlight matches.
- Select a node to show details (path, type, value preview, parent/children).
- Export graph view (PNG/SVG) and copy selected node path.

## JSON-to-Graph Mapping

- Root becomes a single node.
- Object: node with child edges for each key.
- Array: node with child edges for each index.
- Primitive (string/number/boolean/null): leaf node.
- Edge label: key name or index.

## UI Layout

- Left panel: raw JSON + tree view.
- Center: graph canvas.
- Right panel: node details (path, type, value, references).
- Top bar: load, search, filter, export.

## Interaction Details

- Pan/zoom: mouse drag + wheel.
- Drag nodes: optional, with reset layout.
- Expand/collapse: click node or caret.
- Hover: show tooltip with full path and value preview.
- Multi-select: optional for comparing branches.

## Performance Constraints

- Support large JSON by lazy rendering children until expanded.
- Limit label length with ellipsis, show full value on hover.
- Provide max node count warning and progressive loading.

## Checkpoints

### Checkpoint 1: Basic Load + Graph

- Status: In progress (JSON file import + graph/table views)
- Parse JSON input with validation errors
- Build tree model (nodes + edges)
- Render expandable tree list
- Render node-link graph view
- Render full JSON table view
- Hover node shows path/type/value

### Checkpoint 2: Interaction

- Status: In progress
- Pan, zoom, drag
- Expand/collapse nodes
- Hover tooltips
- Reset layout

### Checkpoint 3: Search + Filter

- Status: Planned
- Search by key/value
- Highlight matches and scroll to first
- Filter by type (object/array/primitive)
- Copy JSONPath for selected node

### Checkpoint 4: Export + Performance

- Status: Planned
- Export PNG/SVG
- Lazy render children for large JSON
- Node count warning threshold
- Basic performance profiling notes

## Acceptance Criteria

- Graph matches JSON structure and preserves parent-child relationships.
- Selecting a node always yields a valid JSONPath.
- Large files remain responsive with lazy expansion enabled.
- Exported images capture the current viewport state.
