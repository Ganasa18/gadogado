import { useEffect, useMemo, useState } from "react";
import { dbGetSelectedTables } from "../api";
import type { RagCollection } from "../types";

export function useRagDbCollection(input: {
  selectedCollectionId: number | null;
  collections: RagCollection[];
}) {
  const { selectedCollectionId, collections } = input;
  const [selectedTables, setSelectedTables] = useState<string[]>([]);
  const [isLoadingTables, setIsLoadingTables] = useState(false);

  const currentCollection = useMemo(
    () => collections.find((c) => c.id === selectedCollectionId),
    [collections, selectedCollectionId],
  );

  const isDbCollection = currentCollection?.kind === "db";

  useEffect(() => {
    if (!selectedCollectionId || !isDbCollection) {
      setSelectedTables([]);
      return;
    }

    const loadTables = async () => {
      setIsLoadingTables(true);
      try {
        const tables = await dbGetSelectedTables(selectedCollectionId);
        setSelectedTables(tables);
      } catch (err) {
        console.error("Failed to load selected tables:", err);
        setSelectedTables([]);
      } finally {
        setIsLoadingTables(false);
      }
    };

    void loadTables();
  }, [selectedCollectionId, isDbCollection]);

  return { currentCollection, isDbCollection: !!isDbCollection, selectedTables, isLoadingTables };
}
