import { useState, useRef, useCallback, type KeyboardEvent, type ClipboardEvent } from 'react';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface PathListInputProps {
  value: string[];
  onChange: (paths: string[]) => void;
  placeholder?: string;
  helpText?: string;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function PathListInput({
  value,
  onChange,
  placeholder = '/path/to/directory',
  helpText,
}: PathListInputProps) {
  const [input, setInput] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  const addPaths = useCallback(
    (raw: string) => {
      const next = raw
        .split(',')
        .map((t) => t.trim())
        .filter((t) => t.length > 0 && !value.includes(t));
      if (next.length > 0) {
        onChange([...value, ...next]);
      }
    },
    [value, onChange],
  );

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (input.trim()) {
        addPaths(input);
        setInput('');
      }
    } else if (e.key === 'Backspace' && input === '' && value.length > 0) {
      onChange(value.slice(0, -1));
    }
  }

  function handlePaste(e: ClipboardEvent<HTMLInputElement>) {
    e.preventDefault();
    const pasted = e.clipboardData.getData('text');
    addPaths(pasted);
    setInput('');
  }

  function removePath(index: number) {
    onChange(value.filter((_, i) => i !== index));
  }

  return (
    <div>
      <div
        onClick={() => inputRef.current?.focus()}
        className="flex flex-wrap items-center gap-1.5 rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-2 py-1.5 transition-colors focus-within:border-[var(--accent-hover)] focus-within:ring-2 focus-within:ring-[var(--ring-accent)] cursor-text"
      >
        {value.map((path, i) => (
          <span
            key={`${path}-${i}`}
            className="inline-flex items-center gap-1 rounded-md bg-[var(--bg-primary)] border border-[var(--border-secondary)] px-2 py-0.5 text-xs font-mono text-[var(--text-primary)]"
          >
            {path}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                removePath(i);
              }}
              className="ml-0.5 inline-flex h-3.5 w-3.5 items-center justify-center rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-secondary)] transition-colors"
              aria-label={`Remove ${path}`}
            >
              <svg className="h-2.5 w-2.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={3}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </span>
        ))}
        <input
          ref={inputRef}
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          onBlur={() => {
            if (input.trim()) {
              addPaths(input);
              setInput('');
            }
          }}
          placeholder={value.length === 0 ? placeholder : ''}
          className="min-w-[80px] flex-1 bg-transparent py-0.5 text-sm font-mono text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none"
        />
      </div>
      {helpText && (
        <p className="mt-1 text-xs text-[var(--text-muted)]">{helpText}</p>
      )}
    </div>
  );
}
