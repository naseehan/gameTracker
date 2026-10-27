import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown } from 'lucide-react';

interface Option {
  value: string;
  label: string;
  disabled?: boolean;
}

interface DropdownProps {
  value: string;
  onChange: (value: string) => void;
  options: Option[];
  icon?: React.ReactNode;
}

export function Dropdown({ value, onChange, options, icon }: DropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find(o => o.value === value) || options[0];

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setIsOpen(!isOpen);
    } else if (e.key === 'Escape') {
      setIsOpen(false);
    } else if (isOpen) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        const currentIndex = options.findIndex(o => o.value === value);
        let nextIndex = currentIndex + 1;
        while (nextIndex < options.length && options[nextIndex].disabled) {
          nextIndex++;
        }
        if (nextIndex < options.length) {
          onChange(options[nextIndex].value);
        }
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        const currentIndex = options.findIndex(o => o.value === value);
        let prevIndex = currentIndex - 1;
        while (prevIndex >= 0 && options[prevIndex].disabled) {
          prevIndex--;
        }
        if (prevIndex >= 0) {
          onChange(options[prevIndex].value);
        }
      }
    }
  };

  return (
    <div 
      className="relative flex items-center space-x-2 bg-gray-900 border border-gray-700 rounded-lg p-1"
      ref={dropdownRef}
    >
      {icon && <div className="ml-2">{icon}</div>}
      <button
        type="button"
        className="flex items-center justify-between bg-transparent text-gray-300 text-sm border-none focus:outline-none focus:ring-2 focus:ring-indigo-500 rounded px-2 py-1 flex-1 min-w-[120px]"
        onClick={() => setIsOpen(!isOpen)}
        onKeyDown={handleKeyDown}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
      >
        <span>{selectedOption?.label}</span>
        <ChevronDown size={14} className="ml-2 text-gray-500" />
      </button>

      {isOpen && (
        <ul
          className="absolute z-50 top-full right-0 mt-2 w-48 bg-gray-800 border border-gray-700 rounded-xl shadow-2xl py-1 overflow-hidden"
          role="listbox"
        >
          {options.map((option, index) => {
            if (option.disabled) {
              return (
                <li key={`divider-${index}`} className="px-3 py-1 my-1 flex items-center">
                  <div className="flex-1 border-t border-gray-700/50"></div>
                </li>
              );
            }
            
            const isSelected = value === option.value;
            
            return (
              <li
                key={option.value}
                role="option"
                aria-selected={isSelected}
                className={`px-4 py-2 text-sm cursor-pointer transition-colors flex items-center justify-between ${
                  isSelected 
                    ? 'bg-indigo-600/20 text-indigo-400' 
                    : 'text-gray-300 hover:bg-gray-700 hover:text-white'
                }`}
                onClick={() => {
                  onChange(option.value);
                  setIsOpen(false);
                }}
              >
                {option.label}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}