import { useState, useEffect, type RefObject } from 'react';

interface ContainerSize {
  width: number;
  height: number;
}

export function useContainerSize(ref: RefObject<HTMLDivElement>): ContainerSize {
  const [size, setSize] = useState<ContainerSize>({ width: 0, height: 0 });

  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        setSize({ width, height });
      }
    });

    observer.observe(element);

    // Set initial size
    setSize({
      width: element.clientWidth,
      height: element.clientHeight,
    });

    return () => {
      observer.disconnect();
    };
  }, [ref]);

  return size;
}
