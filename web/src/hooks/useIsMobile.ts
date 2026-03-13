import { useState, useEffect } from 'react';

interface ScreenSize {
  isMobile: boolean;  // < 768px
  isTablet: boolean;  // 768px - 1023px
  isDesktop: boolean; // >= 1024px
}

function getScreenSize(): ScreenSize {
  const width = window.innerWidth;
  return {
    isMobile: width < 768,
    isTablet: width >= 768 && width < 1024,
    isDesktop: width >= 1024,
  };
}

export function useIsMobile(): ScreenSize {
  const [screenSize, setScreenSize] = useState<ScreenSize>(getScreenSize);

  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;

    const handleResize = () => {
      if (timeoutId) clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        setScreenSize(getScreenSize());
      }, 150);
    };

    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
      if (timeoutId) clearTimeout(timeoutId);
    };
  }, []);

  return screenSize;
}
