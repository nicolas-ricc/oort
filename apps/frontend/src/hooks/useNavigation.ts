import { useCallback, useEffect, useRef, useState } from 'react';
import { ConceptCluster } from '@/App';

export type NavigationState = {
  currentIndex: number;
  totalCount: number;
  tourMode: boolean;
  tourInterval: number;
};

export type NavigationActions = {
  navigateToNext: () => void;
  navigateToPrevious: () => void;
  navigateToIndex: (index: number) => void;
  startTour: () => void;
  stopTour: () => void;
  toggleTour: () => void;
  resetToOverview: () => void;
};

function getNodeKey(node: ConceptCluster): string {
  return node.concepts?.slice().sort().join("|") ?? "";
}

export function useNavigation(
  nodes: ConceptCluster[],
  activeNode: string,
  setActive: (key: string) => void,
  tourIntervalMs: number = 3000
): NavigationState & NavigationActions {
  const [tourMode, setTourMode] = useState(false);
  const tourIntervalRef = useRef<NodeJS.Timeout | null>(null);

  // Calculate current index from activeNode
  const currentIndex = nodes.findIndex(node => getNodeKey(node) === activeNode);
  const totalCount = nodes.length;

  const navigateToIndex = useCallback((index: number) => {
    if (index >= 0 && index < nodes.length) {
      const key = getNodeKey(nodes[index]);
      if (key) {
        setActive(key);
      }
    }
  }, [nodes, setActive]);

  const navigateToNext = useCallback(() => {
    const nextIndex = currentIndex >= 0 ? (currentIndex + 1) % totalCount : 0;
    navigateToIndex(nextIndex);
  }, [currentIndex, totalCount, navigateToIndex]);

  const navigateToPrevious = useCallback(() => {
    const prevIndex = currentIndex > 0 ? currentIndex - 1 : totalCount - 1;
    navigateToIndex(prevIndex);
  }, [currentIndex, totalCount, navigateToIndex]);

  const resetToOverview = useCallback(() => {
    // Navigate to first node (or could implement a special "overview" state)
    if (nodes.length > 0) {
      navigateToIndex(0);
    }
  }, [nodes.length, navigateToIndex]);

  const stopTour = useCallback(() => {
    setTourMode(false);
    if (tourIntervalRef.current) {
      clearInterval(tourIntervalRef.current);
      tourIntervalRef.current = null;
    }
  }, []);

  const startTour = useCallback(() => {
    setTourMode(true);
    // Clear any existing interval
    if (tourIntervalRef.current) {
      clearInterval(tourIntervalRef.current);
    }
    // Start auto-advance
    tourIntervalRef.current = setInterval(() => {
      navigateToNext();
    }, tourIntervalMs);
  }, [navigateToNext, tourIntervalMs]);

  const toggleTour = useCallback(() => {
    if (tourMode) {
      stopTour();
    } else {
      startTour();
    }
  }, [tourMode, startTour, stopTour]);

  // Cleanup interval on unmount
  useEffect(() => {
    return () => {
      if (tourIntervalRef.current) {
        clearInterval(tourIntervalRef.current);
      }
    };
  }, []);

  // Update interval when tour mode changes or navigateToNext changes
  useEffect(() => {
    if (tourMode && tourIntervalRef.current) {
      clearInterval(tourIntervalRef.current);
      tourIntervalRef.current = setInterval(() => {
        navigateToNext();
      }, tourIntervalMs);
    }
  }, [tourMode, navigateToNext, tourIntervalMs]);

  return {
    currentIndex: currentIndex >= 0 ? currentIndex : 0,
    totalCount,
    tourMode,
    tourInterval: tourIntervalMs,
    navigateToNext,
    navigateToPrevious,
    navigateToIndex,
    startTour,
    stopTour,
    toggleTour,
    resetToOverview,
  };
}
