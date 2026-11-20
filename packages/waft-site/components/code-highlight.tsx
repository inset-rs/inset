import type { ReactNode } from 'react';

const RUST_KEYWORDS = new Set([
  'use',
  'fn',
  'let',
  'mut',
  'pub',
  'impl',
  'for',
  'trait',
  'struct',
  'enum',
  'async',
  'await',
  'move',
  'return',
  'self',
  'true',
  'false',
  'where',
  'const',
  'static',
  'type',
  'mod',
  'crate',
  'in',
  'if',
  'else',
  'match',
]);

const TOKEN_RE =
  /(\/\/[^\n]*|#!?\[[^\]]*\]|"(?:\\.|[^"\\])*"|\b[a-z_][a-z0-9_]*!|\b[A-Z][A-Za-z0-9_]*\b|\b[a-z_][a-z0-9_]*(?=\s*(?:\(|::<))|\b[a-z_][a-z0-9_]*\b|\b\d+(?:_\d+)*(?:\.\d+)?(?:f32|f64|u32|u64|usize|i32)?\b)/g;

export function highlightRustLine(line: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  TOKEN_RE.lastIndex = 0;

  while ((match = TOKEN_RE.exec(line)) !== null) {
    if (match.index > lastIndex) {
      nodes.push(line.slice(lastIndex, match.index));
    }
    const token = match[0];
    const key = `${match.index}-${token}`;

    if (token.startsWith('//')) {
      nodes.push(
        <span key={key} style={{ color: '#78756e', fontStyle: 'italic' }}>
          {token}
        </span>,
      );
    } else if (token.startsWith('#[' ) || token.startsWith('#![')) {
      nodes.push(
        <span key={key} style={{ color: '#b45309', fontWeight: 500 }}>
          {token}
        </span>,
      );
    } else if (token.startsWith('"')) {
      nodes.push(
        <span key={key} style={{ color: '#15803d' }}>
          {token}
        </span>,
      );
    } else if (token.endsWith('!')) {
      nodes.push(
        <span key={key} style={{ color: '#b45309', fontWeight: 500 }}>
          {token}
        </span>,
      );
    } else if (RUST_KEYWORDS.has(token)) {
      nodes.push(
        <span key={key} style={{ color: '#c84b11', fontWeight: 600 }}>
          {token}
        </span>,
      );
    } else if (/^[A-Z]/.test(token)) {
      nodes.push(
        <span key={key} style={{ color: '#005fb8', fontWeight: 500 }}>
          {token}
        </span>,
      );
    } else if (/^\d/.test(token)) {
      nodes.push(
        <span key={key} style={{ color: '#0f766e' }}>
          {token}
        </span>,
      );
    } else {
      const after = line.slice(match.index + token.length);
      if (/^\s*(?:\(|::<)/.test(after)) {
        nodes.push(
          <span key={key} style={{ color: '#6d28d9', fontWeight: 500 }}>
            {token}
          </span>,
        );
      } else {
        nodes.push(token);
      }
    }

    lastIndex = match.index + token.length;
  }

  if (lastIndex < line.length) {
    nodes.push(line.slice(lastIndex));
  }

  return nodes;
}

export function RustCodeBlock({
  code,
  showLineNumbers = false,
  className = '',
}: {
  code: string;
  showLineNumbers?: boolean;
  className?: string;
}) {
  const lines = code.trim().split('\n');
  return (
    <pre
      className={`overflow-x-auto rounded-xl border p-4 font-mono text-[12.5px] leading-[20px] ${className}`}
      style={{
        background: '#fcfbf9',
        borderColor: 'var(--site-hairline)',
        color: '#1c1b18',
      }}
    >
      <code>
        {lines.map((line, idx) => (
          <div key={idx} className="flex">
            {showLineNumbers && (
              <span
                className="mr-4 inline-block w-5 select-none text-right text-[11px]"
                style={{ color: 'rgba(20, 20, 19, 0.3)' }}
              >
                {idx + 1}
              </span>
            )}
            <span className="whitespace-pre">{line.length > 0 ? highlightRustLine(line) : ' '}</span>
          </div>
        ))}
      </code>
    </pre>
  );
}

export function ShellCodeBlock({ code, className = '' }: { code: string; className?: string }) {
  const lines = code.trim().split('\n');
  return (
    <pre
      className={`overflow-x-auto rounded-xl border p-4 font-mono text-[12.5px] leading-[21px] ${className}`}
      style={{
        background: '#fcfbf9',
        borderColor: 'var(--site-hairline)',
        color: '#1c1b18',
      }}
    >
      <code>
        {lines.map((line, idx) => {
          const trimmed = line.trim();
          if (!trimmed) {
            return (
              <div key={idx} className="h-[10px]">
                {' '}
              </div>
            );
          }
          if (trimmed.startsWith('#')) {
            return (
              <div key={idx} className="whitespace-pre" style={{ color: '#78756e' }}>
                {line}
              </div>
            );
          }
          // Highlight command and trailing comment
          const commentIdx = line.indexOf(' #');
          const cmdPart = commentIdx >= 0 ? line.slice(0, commentIdx) : line;
          const commentPart = commentIdx >= 0 ? line.slice(commentIdx) : '';
          const parts = cmdPart.split(/(\s+)/);

          return (
            <div key={idx} className="whitespace-pre">
              <span className="mr-2 select-none" style={{ color: '#c84b11', fontWeight: 600 }}>
                $
              </span>
              {parts.map((part, pIdx) => {
                if (/^\s+$/.test(part)) return part;
                if (pIdx === 0 || part === 'waft') {
                  return (
                    <span key={pIdx} style={{ color: '#141413', fontWeight: 600 }}>
                      {part}
                    </span>
                  );
                }
                if (part.startsWith('-')) {
                  return (
                    <span key={pIdx} style={{ color: '#005fb8', fontWeight: 500 }}>
                      {part}
                    </span>
                  );
                }
                if (/^(install|new|run|build|test|doctor)$/.test(part)) {
                  return (
                    <span key={pIdx} style={{ color: '#c84b11', fontWeight: 500 }}>
                      {part}
                    </span>
                  );
                }
                return (
                  <span key={pIdx} style={{ color: '#15803d' }}>
                    {part}
                  </span>
                );
              })}
              {commentPart && <span style={{ color: '#78756e' }}>{commentPart}</span>}
            </div>
          );
        })}
      </code>
    </pre>
  );
}
