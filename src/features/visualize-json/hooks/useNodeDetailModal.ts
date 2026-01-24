import { useCallback, useEffect, useState } from "react";

export interface NodeDetail {
  label: string;
  type: string;
  value: unknown;
  path: string;
}

export function useNodeDetailModal() {
  const [nodeDetail, setNodeDetail] = useState<NodeDetail | null>(null);

  useEffect(() => {
    const handleShowNodeDetail = (event: Event) => {
      const customEvent = event as CustomEvent<NodeDetail>;
      setNodeDetail(customEvent.detail);
    };

    window.addEventListener("showNodeDetail", handleShowNodeDetail);
    return () => {
      window.removeEventListener("showNodeDetail", handleShowNodeDetail);
    };
  }, []);

  const closeNodeDetail = useCallback(() => {
    setNodeDetail(null);
  }, []);

  return { nodeDetail, closeNodeDetail };
}
