import { useEffect } from "react";

export function useToggleNodeEvent(toggleNode: (path: string) => void) {
  useEffect(() => {
    const handleToggleNode = (event: Event) => {
      const customEvent = event as CustomEvent<string>;
      toggleNode(customEvent.detail);
    };

    window.addEventListener("toggleNode", handleToggleNode);
    return () => {
      window.removeEventListener("toggleNode", handleToggleNode);
    };
  }, [toggleNode]);
}
