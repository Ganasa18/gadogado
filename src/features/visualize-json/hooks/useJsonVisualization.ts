import { useState, useEffect, useCallback } from 'react';
import { JsonNode, VisualizationState, LocalStorageData, HistoryItem } from '../types';
import * as yaml from 'js-yaml';

const VISUALIZE_JSON_KEY = 'visualize-json-data';

export const useJsonVisualization = () => {
  const [state, setState] = useState<VisualizationState>({
    json: null,
    history: [],
    currentPath: ''
  });

  const loadFromLocalStorage = useCallback(() => {
    try {
      const stored = localStorage.getItem(VISUALIZE_JSON_KEY);
      if (stored) {
        const data = JSON.parse(stored) as LocalStorageData & {
          visualizations?: VisualizationState[];
        };
        const visualization = data.visualization ?? data.visualizations?.[0];
        if (visualization) {
          const history = Array.isArray(visualization.history) 
            ? visualization.history.map((h: any) => ({
                id: h.id || Date.now().toString(),
                data: h.data || h,
                format: h.format || 'json',
                filename: h.filename || 'Untitled',
                timestamp: h.timestamp || new Date().toISOString(),
                content: h.content || (typeof h.data === 'string' ? h.data : JSON.stringify(h.data))
              }))
            : [];
          
          setState({
            json: visualization.json,
            history,
            currentPath: visualization.currentPath
          });
        }
      }
    } catch (error) {
      console.error('Failed to load from localStorage:', error);
      console.log('Clearing localStorage due to error');
      localStorage.removeItem(VISUALIZE_JSON_KEY);
      setState({ json: null, history: [], currentPath: '' });
    }
  }, []);

  const saveToLocalStorage = useCallback((newState: VisualizationState) => {
    try {
      const data: LocalStorageData = {
        visualization: newState,
        lastOpened: new Date().toISOString()
      };
      localStorage.setItem(VISUALIZE_JSON_KEY, JSON.stringify(data));
      setState(prevState => ({ ...prevState, ...newState }));
    } catch (error) {
      console.error('Failed to save to localStorage:', error);
    }
  }, []);

  const clearLocalStorage = useCallback(() => {
    try {
      localStorage.removeItem(VISUALIZE_JSON_KEY);
      setState({ json: null, history: [], currentPath: '' });
    } catch (error) {
      console.error('Failed to clear localStorage:', error);
    }
  }, []);

  const parseJson = (jsonString: string): JsonNode | null => {
    try {
      const parsed = JSON.parse(jsonString);
      return buildJsonTree(parsed, '$', 0, 'root');
    } catch (error) {
      console.error('Invalid JSON:', error);
      return null;
    }
  };

  const parseYaml = (yamlString: string): JsonNode | null => {
    try {
      const parsed = yaml.load(yamlString);
      return buildJsonTree(parsed, '$', 0, 'root');
    } catch (error) {
      console.error('Invalid YAML:', error);
      return null;
    }
  };

  const parseToml = (tomlString: string): JsonNode | null => {
    try {
      const lines = tomlString.split('\n');
      const result: any = {};
      let currentSection: any = result;

      for (const line of lines) {
        const trimmed = line.trim();
        
        if (!trimmed || trimmed.startsWith('#')) continue;
        
        const sectionMatch = trimmed.match(/^\[([^\]]+)\]$/);
        if (sectionMatch) {
          const keys = sectionMatch[1].split('.');
          currentSection = result;
          for (const key of keys) {
            if (!currentSection[key]) {
              currentSection[key] = {};
            }
            currentSection = currentSection[key];
          }
          continue;
        }

        const kvMatch = trimmed.match(/^([^=]+)=(.+)$/);
        if (kvMatch) {
          const key = kvMatch[1].trim();
          let value: any = kvMatch[2].trim();
          
          if (value.startsWith('"') && value.endsWith('"')) {
            value = value.slice(1, -1);
          } else if (value === 'true') {
            value = true;
          } else if (value === 'false') {
            value = false;
          } else if (!isNaN(Number(value))) {
            value = Number(value);
          }

          currentSection[key] = value;
        }
      }

      return buildJsonTree(result, '$', 0, 'root');
    } catch (error) {
      console.error('Invalid TOML:', error);
      return null;
    }
  };

  const parseXml = (xmlString: string): JsonNode | null => {
    try {
      const parser = new DOMParser();
      const xmlDoc = parser.parseFromString(xmlString, 'text/xml');
      const errorNode = xmlDoc.querySelector('parsererror');
      if (errorNode) {
        console.error('Invalid XML:', errorNode.textContent);
        return null;
      }
      const xmlToJson = (node: Element | Text, path: string, depth: number, key: string): JsonNode => {
        if (node.nodeType === Node.TEXT_NODE) {
          const textNode = node as Text;
          if (textNode.nodeValue?.trim()) {
            return {
              key,
              value: textNode.nodeValue.trim(),
              type: 'string',
              path,
              depth
            };
          }
        }

        const element = node as Element;
        const jsonNode: JsonNode = {
          key,
          value: null,
          type: 'object',
          path,
          depth,
          children: [],
          expanded: depth < 2
        };

        const children: JsonNode[] = [];
        Array.from(element.childNodes).forEach((child) => {
          if (child.nodeType === Node.ELEMENT_NODE) {
            const childElement = child as Element;
            const childPath = `${path}.${childElement.tagName}`;
            const childNode = xmlToJson(childElement, childPath, depth + 1, childElement.tagName);
            children.push(childNode);
          } else if (child.nodeType === Node.TEXT_NODE) {
            const textNode = child as Text;
            if (textNode.nodeValue?.trim()) {
              children.push({
                key: 'value',
                value: textNode.nodeValue.trim(),
                type: 'string',
                path: `${path}.value`,
                depth: depth + 1
              });
            }
          }
        });

        if (children.length === 1 && children[0].type === 'string' && children[0].key === 'value') {
          return {
            key,
            value: children[0].value,
            type: 'string',
            path,
            depth
          };
        }

        jsonNode.children = children;
        return jsonNode;
      };

      return xmlToJson(xmlDoc.documentElement, '$', 0, xmlDoc.documentElement.tagName);
    } catch (error) {
      console.error('Invalid XML:', error);
      return null;
    }
  };

  const parseCsv = (csvString: string): JsonNode | null => {
    try {
      const lines = csvString.trim().split('\n').filter(line => line.trim());
      if (lines.length === 0) {
        console.error('Empty CSV');
        return null;
      }

      const firstLine = lines[0];
      const semicolonCount = (firstLine.match(/;/g) || []).length;
      const commaCount = (firstLine.match(/,/g) || []).length;
      const delimiter = semicolonCount > commaCount ? ';' : ',';

      const headers = firstLine.split(delimiter).map((h: string) => h.trim().replace(/^"|"$/g, ''));
      const data: any[] = [];
      
      for (let i = 1; i < lines.length; i++) {
        const values = lines[i].split(delimiter).map((v: string) => v.trim().replace(/^"|"$/g, ''));
        const row: any = {};
        headers.forEach((header: string, index: number) => {
          row[header] = values[index] || '';
        });
        data.push(row);
      }

      return buildJsonTree(data, '$', 0, 'root');
    } catch (error) {
      console.error('Invalid CSV:', error);
      return null;
    }
  };

  const parseData = (content: string, format: HistoryItem['format']): JsonNode | null => {
    switch (format) {
      case 'json':
        return parseJson(content);
      case 'yaml':
        return parseYaml(content);
      case 'toml':
        return parseToml(content);
      case 'xml':
        return parseXml(content);
      case 'csv':
        return parseCsv(content);
      default:
        return parseJson(content);
    }
  };

  const createHistoryItem = (data: any, format: 'json' | 'csv' | 'xml' | 'yaml' | 'toml', filename?: string, content?: string) => {
    const newItem = {
      id: Date.now().toString(),
      data,
      format,
      filename,
      timestamp: new Date().toISOString(),
      content: content || (typeof data === 'string' ? data : JSON.stringify(data))
    };
    return newItem;
  };

  const loadFromHistory = (itemId: string) => {
    try {
      const item = state.history.find((h: any) => h.id === itemId);
      if (item) {
        const content = item.content || JSON.stringify(item.data);
        const jsonNode = parseData(content, item.format || 'json');
        if (jsonNode) {
          saveToLocalStorage({ ...state, json: jsonNode });
        } else {
          alert('Error loading item from history. The data may be corrupted.');
        }
      }
    } catch (error) {
      console.error('Failed to load from history:', error);
      alert('Error loading item from history. The data may be corrupted.');
    }
  };

  const removeFromHistory = (itemId: string) => {
    const updatedHistory = state.history.filter((h: any) => h.id !== itemId);
    saveToLocalStorage({ ...state, history: updatedHistory });
  };

  const buildJsonTree = (
    data: any,
    path: string,
    depth: number,
    key: string
  ): JsonNode => {
    const type = getType(data);

    if (type === 'object') {
      const children = Object.entries(data).map(([childKey, value]) =>
        buildJsonTree(value, `${path}.${childKey}`, depth + 1, childKey)
      );

      return {
        key,
        value: data,
        type,
        path,
        depth,
        children,
        expanded: depth < 2
      };
    }

    if (type === 'array') {
      const children = data.map((value: any, index: number) =>
        buildJsonTree(value, `${path}[${index}]`, depth + 1, `[${index}]`)
      );

      return {
        key,
        value: data,
        type,
        path,
        depth,
        children,
        expanded: depth < 2
      };
    }

    return {
      key,
      value: data,
      type,
      path,
      depth
    };
  };

  const getType = (value: any): JsonNode['type'] => {
    if (value === null) return 'null';
    if (Array.isArray(value)) return 'array';
    if (typeof value === 'object') return 'object';
    if (typeof value === 'string') return 'string';
    if (typeof value === 'number') return 'number';
    if (typeof value === 'boolean') return 'boolean';
    return 'null';
  };

  const toggleNode = (nodePath: string) => {
    const updateNode = (node: JsonNode): JsonNode => {
      if (node.path === nodePath) {
        return { ...node, expanded: !node.expanded };
      }
      if (node.children) {
        return {
          ...node,
          children: node.children.map(updateNode)
        };
      }
      return node;
    };
    
    if (state.json) {
      const updatedJson = updateNode(state.json);
      saveToLocalStorage({ ...state, json: updatedJson });
    }
  };

  useEffect(() => {
    loadFromLocalStorage();
  }, [loadFromLocalStorage]);

  return {
    state,
    loadFromLocalStorage,
    saveToLocalStorage,
    clearLocalStorage,
    parseJson,
    parseData,
    createHistoryItem,
    loadFromHistory,
    removeFromHistory,
    toggleNode
  };
};
