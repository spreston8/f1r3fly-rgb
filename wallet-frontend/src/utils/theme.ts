import { useState, useEffect } from 'react';

const getInitialTheme = (): boolean => {
  const saved = localStorage.getItem('theme');
  const isDark = saved ? saved === 'dark' : true; // Default to dark mode
  
  // Apply immediately to avoid flash
  if (isDark) {
    document.documentElement.classList.add('dark');
  } else {
    document.documentElement.classList.remove('dark');
  }
  
  return isDark;
};

export const useTheme = () => {
  const [isDark, setIsDark] = useState(getInitialTheme);

  useEffect(() => {
    const root = document.documentElement;
    if (isDark) {
      root.classList.add('dark');
      localStorage.setItem('theme', 'dark');
    } else {
      root.classList.remove('dark');
      localStorage.setItem('theme', 'light');
    }
  }, [isDark]);

  const toggleTheme = () => setIsDark(!isDark);

  return { isDark, toggleTheme };
};

