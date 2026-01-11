import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface HistoryItem {
  id: string;
  timestamp: number;
  type: 'translation' | 'enhancement' | 'typegen';
  input: string;
  output: string;
  provider: string;
  model: string;
  topic?: string;
  feedback?: string;
}

export interface HistoryState {
  items: HistoryItem[];
  addItem: (item: Omit<HistoryItem, 'id' | 'timestamp'>) => void;
  clearHistory: () => void;
  removeItem: (id: string) => void;
  setFeedback: (id: string, feedback: string) => void;
}

export const useHistoryStore = create<HistoryState>()(
  persist(
    (set) => ({
      items: [],
      addItem: (item) => set((state) => ({
        items: [
          {
            ...item,
            id: crypto.randomUUID(),
            timestamp: Date.now(),
            topic: item.topic ?? deriveTopic(item.input),
            feedback: item.feedback ?? '',
          },
          ...state.items,
        ],
      })),
      clearHistory: () => set({ items: [] }),
      removeItem: (id) => set((state) => ({
        items: state.items.filter((i) => i.id !== id),
      })),
      setFeedback: (id, feedback) => set((state) => ({
        items: state.items.map((item) =>
          item.id === id ? { ...item, feedback } : item
        ),
      })),
    }),
    {
      name: 'promptbridge-history',
    }
  )
);

function deriveTopic(input: string) {
  const cleaned = input.replace(/\s+/g, ' ').trim();
  if (!cleaned) return 'General';
  const words = cleaned.split(' ');
  const snippet = words.slice(0, 6).join(' ');
  return words.length > 6 ? `${snippet}...` : snippet;
}
