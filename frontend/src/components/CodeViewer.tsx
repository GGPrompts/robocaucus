import { useEffect, useState, useMemo, useCallback } from 'react';
import { codeToHtml, bundledLanguages } from 'shiki';
import { createCssVariablesTheme } from 'shiki';
import { fetchFileContent } from '../lib/api';

// ── Theme ─────────────────────────────────────────────────────────────────

const cssVarsTheme = createCssVariablesTheme({
  name: 'css-variables',
  variablePrefix: '--shiki-',
  variableDefaults: {},
  fontStyle: true,
});

// ── Language detection ────────────────────────────────────────────────────

const extensionToLanguage: Record<string, string> = {
  // JavaScript/TypeScript
  js: 'javascript', jsx: 'jsx', ts: 'typescript', tsx: 'tsx',
  mjs: 'javascript', cjs: 'javascript',
  // Web
  html: 'html', htm: 'html', css: 'css', scss: 'scss', sass: 'sass',
  less: 'less', vue: 'vue', svelte: 'svelte',
  // Backend
  py: 'python', rb: 'ruby', php: 'php', java: 'java',
  kt: 'kotlin', kts: 'kotlin', scala: 'scala', go: 'go', rs: 'rust',
  c: 'c', cpp: 'cpp', cc: 'cpp', h: 'c', hpp: 'cpp', cs: 'csharp', swift: 'swift',
  // Shell
  sh: 'bash', bash: 'bash', zsh: 'bash', fish: 'fish',
  ps1: 'powershell', bat: 'bat', cmd: 'bat',
  // Config/Data
  json: 'json', jsonc: 'jsonc', yaml: 'yaml', yml: 'yaml',
  toml: 'toml', ini: 'ini', xml: 'xml',
  // Markdown
  md: 'markdown', mdx: 'mdx',
  // SQL
  sql: 'sql',
  // Docker
  dockerfile: 'dockerfile',
  // Other
  graphql: 'graphql', gql: 'graphql', lua: 'lua', r: 'r', R: 'r',
  perl: 'perl', pl: 'perl', hs: 'haskell', elm: 'elm', clj: 'clojure',
  ex: 'elixir', exs: 'elixir', erl: 'erlang',
  make: 'makefile', Makefile: 'makefile', cmake: 'cmake',
  vim: 'viml', tex: 'latex', diff: 'diff', prisma: 'prisma', astro: 'astro',
};

function getLanguageFromPath(filePath: string): string {
  const fileName = filePath.split('/').pop() || '';

  if (fileName === 'Dockerfile' || fileName.startsWith('Dockerfile.')) return 'dockerfile';
  if (fileName === 'Makefile' || fileName === 'makefile') return 'makefile';
  if (fileName === '.gitignore' || fileName === '.dockerignore') return 'gitignore';
  if (fileName === '.env' || fileName.startsWith('.env.')) return 'dotenv';

  const ext = fileName.split('.').pop()?.toLowerCase() || '';
  const lang = extensionToLanguage[ext];

  if (lang && lang in bundledLanguages) {
    return lang;
  }

  return 'text';
}

// ── Props ─────────────────────────────────────────────────────────────────

interface CodeViewerProps {
  /** Relative file path within basePath */
  filePath: string;
  /** Root workspace directory */
  basePath: string;
}

// ── Component ─────────────────────────────────────────────────────────────

export function CodeViewer({ filePath, basePath }: CodeViewerProps) {
  const [content, setContent] = useState<string>('');
  const [highlightedHtml, setHighlightedHtml] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isBinary, setIsBinary] = useState(false);

  const language = useMemo(() => getLanguageFromPath(filePath), [filePath]);

  // Fetch file content
  useEffect(() => {
    let cancelled = false;

    async function load() {
      setIsLoading(true);
      setError(null);
      setIsBinary(false);

      try {
        const res = await fetchFileContent(basePath, filePath);
        if (cancelled) return;

        if (res.is_binary) {
          setIsBinary(true);
          setContent('');
          setHighlightedHtml('');
        } else {
          setContent(res.content);
          setIsBinary(false);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to load file');
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    }

    load();
    return () => { cancelled = true; };
  }, [filePath, basePath]);

  // Syntax highlight when content changes
  useEffect(() => {
    if (!content || isBinary) {
      setHighlightedHtml('');
      return;
    }

    let cancelled = false;

    async function highlight() {
      try {
        const html = await codeToHtml(content, {
          lang: language,
          theme: cssVarsTheme,
        });
        if (!cancelled) {
          setHighlightedHtml(html);
        }
      } catch (err) {
        console.error('Shiki highlighting error:', err);
        if (!cancelled) {
          setHighlightedHtml('');
        }
      }
    }

    highlight();
    return () => { cancelled = true; };
  }, [content, language, isBinary]);

  const lineCount = useMemo(() => content.split('\n').length, [content]);

  const getLineStyles = useCallback((_lineNum: number): { gutter: React.CSSProperties; content: React.CSSProperties } => {
    return { gutter: {}, content: {} };
  }, []);

  // Loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full" style={{ color: 'var(--text-secondary)' }}>
        <p>Loading...</p>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="flex items-center justify-center h-full" style={{ color: 'var(--text-secondary)' }}>
        <p style={{ color: 'var(--error, #ef4444)' }}>{error}</p>
      </div>
    );
  }

  // Binary file
  if (isBinary) {
    return (
      <div className="flex items-center justify-center h-full" style={{ color: 'var(--text-secondary)' }}>
        <p>Binary file cannot be displayed</p>
      </div>
    );
  }

  // Fallback for no highlighted HTML yet
  if (!highlightedHtml) {
    return (
      <div className="code-viewer h-full overflow-auto">
        <div className="flex">
          <div
            className="line-numbers select-none text-right pr-4 pl-4 py-4"
            style={{
              color: 'var(--text-secondary)',
              backgroundColor: 'var(--bg-secondary)',
              fontFamily: 'var(--font-mono)',
              fontSize: '0.875rem',
              lineHeight: '1.7',
              minWidth: '3rem',
              borderRight: '1px solid var(--border)',
            }}
          >
            {Array.from({ length: lineCount }, (_, i) => i + 1).map((num) => {
              const styles = getLineStyles(num);
              return (
                <div key={num} data-line={num} style={styles.gutter}>
                  {num}
                </div>
              );
            })}
          </div>
          <pre
            className="flex-1 p-4 m-0 overflow-x-auto"
            style={{
              backgroundColor: 'var(--bg-secondary)',
              color: 'var(--text-primary)',
              fontFamily: 'var(--font-mono)',
              fontSize: '0.875rem',
              lineHeight: '1.7',
            }}
          >
            <code>{content}</code>
          </pre>
        </div>
      </div>
    );
  }

  // Highlighted view
  return (
    <div className="code-viewer h-full" style={{ position: 'relative' }}>
      <div className="flex">
        <div
          className="line-numbers select-none py-4 sticky left-0"
          style={{
            color: 'var(--text-secondary)',
            backgroundColor: 'var(--bg-secondary)',
            fontFamily: 'var(--font-mono)',
            fontSize: '0.875rem',
            lineHeight: '1.7',
            minWidth: '3rem',
            borderRight: '1px solid var(--border)',
          }}
        >
          {Array.from({ length: lineCount }, (_, i) => i + 1).map((num) => {
            const styles = getLineStyles(num);
            return (
              <div
                key={num}
                data-line={num}
                style={{
                  ...styles.gutter,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'flex-end',
                  paddingLeft: '0.5rem',
                  paddingRight: '1rem',
                }}
              >
                <span style={{ minWidth: '2ch', textAlign: 'right' }}>{num}</span>
              </div>
            );
          })}
        </div>
        <div
          className="code-content flex-1 overflow-x-auto"
          style={{ backgroundColor: 'var(--bg-secondary)' }}
        >
          <style>{`
            .code-content pre {
              margin: 0;
              padding: 1rem;
              font-family: var(--font-mono);
              font-size: 0.875rem;
              line-height: 1.7;
            }
            .code-content code {
              font-family: inherit;
              font-size: inherit;
              line-height: inherit;
            }
            .code-content .line {
              display: block;
              min-height: 1.7em;
            }
          `}</style>
          <div dangerouslySetInnerHTML={{ __html: highlightedHtml }} />
        </div>
      </div>
    </div>
  );
}
