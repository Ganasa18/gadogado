import { useEffect, useState } from "react";
import { listRagCollections } from "../api";
import type { RagCollection } from "../types";

export function useRagCollections() {
  const [collections, setCollections] = useState<RagCollection[]>([]);

  useEffect(() => {
    listRagCollections(50).then(setCollections).catch(console.error);
  }, []);

  return { collections };
}
